#![allow(non_camel_case_types)]

use citeproc::prelude::*;
use csl::{Lang, Locale};

#[macro_use]
mod macros;

use libc::{c_char, c_void};
use std::ffi::{CStr, CString};
use std::sync::Arc;

/// Wrapper for a Processor, initialized with one style and any required locales
pub struct citeproc_rs(Processor);

type citeproc_fetch_locale_callback =
    Option<unsafe extern "C" fn(context: *mut c_void, slot: *mut LocaleSlot, *const c_char)>;

pub struct LocaleSlot {
    storage: *mut LocaleStorage,
    lang: *const Lang,
}

struct FFILocaleFetcher {
    context: *mut c_void,
    callback: citeproc_fetch_locale_callback,
    storage: LocaleStorage,
}

impl FFILocaleFetcher {
    /// get `to_fetch` from `Processor::get_langs_in_use()`
    fn build(mut self, to_fetch: &[Lang]) -> LocaleStorage {
        use std::io::Write;
        let mut string_repr = Vec::<u8>::with_capacity(20);
        for lang in to_fetch {
            if *lang == Lang::en_us() {
                continue;
            }
            write!(&mut string_repr, "{}\0", lang).expect("in-memory write should not fail");
            let lang_str_ref = CStr::from_bytes_with_nul(string_repr.as_ref())
                .expect("definitely formatted this CStr with a null byte?");
            let mut slot = LocaleSlot {
                storage: &mut self.storage,
                lang,
            };
            if let Some(callback) = self.callback {
                unsafe {
                    callback(self.context, &mut slot, lang_str_ref.as_ptr());
                }
            }
            string_repr.clear();
        }
        self.storage
    }
}

struct LocaleStorage {
    locales: Vec<(Lang, String)>,
}

ffi_fn! {
    fn citeproc_rs_write_locale_slot(slot: *mut LocaleSlot, locale_xml: *const c_char, locale_xml_len: usize) {
        // Safety: we asked for this to be passed back transparently.
        let slot = unsafe { &mut *slot };
        // Safety: we asked folks to give us an XML string.
        let locale_xml = unsafe { utf8_from_raw!(locale_xml, locale_xml_len) };
        // We'll parse it here as well so you catch errors before they become invisible as
        // mysteriously missing locales
        let _ = Locale::parse(locale_xml).expect("could not parse locale xml");
        // Safety: we control slot
        let storage = unsafe { &mut *slot.storage };
        let lang = unsafe { &*slot.lang };
        storage.locales.push((lang.clone(), locale_xml.to_owned()));
        // println!("added locale for lang: {}", lang);
    }
}

// impl LocaleFetcher for FFILocaleFetcher {
//     fn fetch_locale(&self, lang: &Lang) -> Option<Locale> {
//     }
//
//     fn fetch_string(&self, lang: &Lang) -> Result<Option<String>, LocaleFetchError> {
//     }
// }

// fn utf8_from_raw<'a>(style: &'a *const c_char, style_len: usize) -> &'a str {
// }

#[repr(u8)]
pub enum citeproc_rs_output_format {
    HTML = 0,
    RTF = 1,
    PLAIN = 2,
}

#[repr(C)]
pub struct citeproc_rs_init_options {
    style: *const c_char,
    style_len: usize,
    locale_fetch_context: *mut libc::c_void,
    locale_fetch_callback: citeproc_fetch_locale_callback,
    format: citeproc_rs_output_format,
}

ffi_fn! {
    fn citeproc_rs_new(init: citeproc_rs_init_options) -> *mut citeproc_rs {
        let style = unsafe { utf8_from_raw!(init.style, init.style_len) };
        let rs_init = InitOptions {
            format: match init.format {
                citeproc_rs_output_format::HTML => SupportedFormat::Html,
                citeproc_rs_output_format::RTF => SupportedFormat::Rtf,
                citeproc_rs_output_format::PLAIN => SupportedFormat::Plain,
            },
            style,
            fetcher: Some(Arc::new(PredefinedLocales::bundled_en_us())),
            ..Default::default()
        };
        let mut proc = match Processor::new(rs_init) {
            Ok(p) => p,
            Err(e) => panic!("{}", e),
        };
        let langs = proc.get_langs_in_use();
        if !langs.is_empty() {
            if let Some(_) = init.locale_fetch_callback {
                let ffi_locales = FFILocaleFetcher {
                    callback: init.locale_fetch_callback,
                    context: init.locale_fetch_context,
                    storage: LocaleStorage { locales: Vec::with_capacity(langs.len()) },
                };
                let locales = ffi_locales.build(&langs).locales;
                proc.store_locales(locales)
            }
        }
        Box::into_raw(Box::new(citeproc_rs(proc)))
    }
}

ffi_fn! {
    fn citeproc_rs_free(ptr: *mut citeproc_rs) {
        if !ptr.is_null() {
            drop(unsafe { Box::from_raw(ptr) });
        }
    }
}

ffi_fn! {
    /// Frees a string returned from citeproc_rs_ API.
    fn citeproc_rs_string_free(ptr: *mut c_char) {
        if !ptr.is_null() {
            drop(unsafe { CString::from_raw(ptr) });
        }
    }
}

ffi_fn! {
    /// let reference: [String: Any] = [ "id": "blah", "type": "book", ... ];
    /// in Swift, JSONSerialization.data(reference).withUnsafeBytes({ rBytes in
    ///     citeproc_rs_format_one(processor, rBytes.baseAddress, rBytes.count)
    /// })
    ///
    /// May return null.
    fn citeproc_rs_format_one(processor: *mut citeproc_rs, ref_bytes: *const c_char, ref_bytes_len: usize) -> *mut c_char {
        let ref_json = unsafe { utf8_from_raw!(ref_bytes, ref_bytes_len) };
        let reference: Reference = serde_json::from_str(ref_json).unwrap();
        let id = reference.id.clone();
        let proc = unsafe { &mut (*processor).0 };
        proc.insert_reference(reference);
        let cluster = proc.preview_cluster_id();
        let result = proc.preview_citation_cluster(
            &[Cite::basic(id)],
            PreviewPosition::MarkWithZero(&[ClusterPosition { id: cluster, note: None }]),
            None,
        );
        if let Ok(result) = result {
            let c = CString::new(result.as_bytes()).unwrap();
            c.into_raw()
        } else {
            std::ptr::null_mut()
        }
    }
}

