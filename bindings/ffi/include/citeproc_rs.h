#ifndef _CITEPROC_RS_H
#define _CITEPROC_RS_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

enum citeproc_rs_output_format
#ifdef __cplusplus
  : uint8_t
#endif // __cplusplus
 {
  HTML = 0,
  RTF = 1,
  PLAIN = 2,
};
#ifndef __cplusplus
typedef uint8_t citeproc_rs_output_format;
#endif // __cplusplus

typedef struct LocaleSlot LocaleSlot;

/*
 Wrapper for a Processor, initialized with one style and any required locales
 */
typedef struct citeproc_rs citeproc_rs;

typedef void (*citeproc_fetch_locale_callback)(void *context, struct LocaleSlot *slot, const char*);

typedef struct citeproc_rs_init_options {
  const char *style;
  uintptr_t style_len;
  void *locale_fetch_context;
  citeproc_fetch_locale_callback locale_fetch_callback;
  citeproc_rs_output_format format;
} citeproc_rs_init_options;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

void citeproc_rs_write_locale_slot(struct LocaleSlot *slot,
                                   const char *locale_xml,
                                   uintptr_t locale_xml_len);

struct citeproc_rs *citeproc_rs_new(struct citeproc_rs_init_options init);

void citeproc_rs_free(struct citeproc_rs *ptr);

/*
 Frees a string returned from citeproc_rs_ API.
 */
void citeproc_rs_string_free(char *ptr);

/*
 let reference: [String: Any] = [ "id": "blah", "type": "book", ... ];
 in Swift, JSONSerialization.data(reference).withUnsafeBytes({ rBytes in
     citeproc_rs_format_one(processor, rBytes.baseAddress, rBytes.count)
 })

 May return null.
 */
char *citeproc_rs_format_one(struct citeproc_rs *processor,
                             const char *ref_bytes,
                             uintptr_t ref_bytes_len);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* _CITEPROC_RS_H */
