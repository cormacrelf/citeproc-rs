#include <stdlib.h>
#include <stdio.h>
#include <assert.h>
#include <string.h>

#include "citeproc_rs.h"

// length excluding null terminator
#define STRLEN(s) (sizeof(s)/sizeof(s[0]) - 1)
#define LIT_LEN(name, lit) const char *name = (lit); uintptr_t name##_len = STRLEN(lit)

LIT_LEN(style, "<style xmlns=\"http://purl.org/net/xbiblio/csl\" class=\"note\" version=\"1.0\" default-locale=\"en-GB\">"
               "<info><id>id</id><title>title</title><updated>2015-10-10T23:31:02+00:00</updated></info>"
               "<citation><layout><text variable=\"title\" /></layout></citation></style>");

LIT_LEN(en_us, "<locale version=\"1.0\" xml:lang=\"en-US\">\n"
                "<info> <updated>2015-10-10T23:31:02+00:00</updated> </info>"
                "<terms> </terms>"
                "</locale>");

void locale_fetch_callback(void *context, LocaleSlot *slot, const char *lang) {
        printf("context carried: %s\n", *((char **)context));
        citeproc_rs_write_locale_slot(slot, en_us, en_us_len);
}

int main() {
        char *context_ex = "example context";
        void *context = (void *) &context_ex;
        citeproc_rs_init_options init = {
                .style = style,
                .style_len = style_len,
                .locale_fetch_context = context,
                .locale_fetch_callback = locale_fetch_callback,
                .format = HTML,
        };
        citeproc_rs *proc = citeproc_rs_new(init);

        const char *ref_json = "{"
                "\"id\": \"item\","
                "\"type\": \"book\","
                "\"title\": \"the title\""
        "}";
        size_t ref_json_len = strlen(ref_json);
        char *result = citeproc_rs_format_one(proc, ref_json, ref_json_len);
        if (result) {
                assert(strcmp(result, "the title") == 0);
                printf("success: %s\n", result);
        }
        citeproc_rs_string_free(result);
        citeproc_rs_free(proc);
}
