/*
 * idn2.h: the part of the libidn2 C interface this library provides.
 *
 * An independent implementation: host names are converted to their ASCII form
 * by UTS #46 non-transitional processing followed by the IDNA2008 lookup
 * checks (RFC 5891, 5892, 5893), over the IDNA data of Unicode 15.1.0 and the
 * character properties of Unicode 16.0.0, answering as GNU libidn2 2.3.8 does.
 */
#ifndef IDN2_H
#define IDN2_H

#ifdef __cplusplus
extern "C" {
#endif

/* Flags of idn2_to_ascii_8z. This library accepts 0 and any union of
   IDN2_NFC_INPUT, IDN2_ALABEL_ROUNDTRIP, IDN2_NONTRANSITIONAL and
   IDN2_ALLOW_UNASSIGNED, which all select the same conversion; every other
   flag is answered with IDN2_INVALID_FLAGS. */
enum {
  IDN2_NFC_INPUT = 1,
  IDN2_ALABEL_ROUNDTRIP = 2,
  IDN2_TRANSITIONAL = 4,
  IDN2_NONTRANSITIONAL = 8,
  IDN2_ALLOW_UNASSIGNED = 16,
  IDN2_USE_STD3_ASCII_RULES = 32,
  IDN2_NO_TR46 = 64,
  IDN2_NO_ALABEL_ROUNDTRIP = 128
};

/* Return codes. */
enum {
  IDN2_OK = 0,
  IDN2_MALLOC = -100,
  IDN2_NO_CODESET = -101,
  IDN2_ICONV_FAIL = -102,
  IDN2_ENCODING_ERROR = -200,
  IDN2_NFC = -201,
  IDN2_PUNYCODE_BAD_INPUT = -202,
  IDN2_PUNYCODE_BIG_OUTPUT = -203,
  IDN2_PUNYCODE_OVERFLOW = -204,
  IDN2_TOO_BIG_DOMAIN = -205,
  IDN2_TOO_BIG_LABEL = -206,
  IDN2_INVALID_ALABEL = -207,
  IDN2_UALABEL_MISMATCH = -208,
  IDN2_INVALID_FLAGS = -209,
  IDN2_NOT_NFC = -300,
  IDN2_2HYPHEN = -301,
  IDN2_HYPHEN_STARTEND = -302,
  IDN2_LEADING_COMBINING = -303,
  IDN2_DISALLOWED = -304,
  IDN2_CONTEXTJ = -305,
  IDN2_CONTEXTJ_NO_RULE = -306,
  IDN2_CONTEXTO = -307,
  IDN2_CONTEXTO_NO_RULE = -308,
  IDN2_UNASSIGNED = -309,
  IDN2_BIDI = -310,
  IDN2_DOT_IN_LABEL = -311,
  IDN2_INVALID_TRANSITIONAL = -312,
  IDN2_INVALID_NONTRANSITIONAL = -313,
  IDN2_ALABEL_ROUNDTRIP_FAILED = -314
};

/* Converts the zero-terminated UTF-8 host name `input` to its ASCII form.
   On IDN2_OK, *output is a zero-terminated string the caller releases with
   idn2_free (NULL when input is NULL). On any other code *output is left
   untouched. */
int idn2_to_ascii_8z(const char *input, char **output, int flags);

/* A constant English description of a return code. */
const char *idn2_strerror(int rc);

/* Releases a string idn2_to_ascii_8z returned. */
void idn2_free(void *ptr);

#ifdef __cplusplus
}
#endif

#endif
