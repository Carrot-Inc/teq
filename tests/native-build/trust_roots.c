/* The native-build suite's check of the trust roots' wraps of src/native/static_musl.c (tests/native-build.scala):
   linked with static_musl.c as a build links it, with -Wl,-wrap=X509_STORE_set_default_paths
   -Wl,-wrap=X509_verify_cert, over the smallest stand-in for libcrypto that the wraps can run on
   (libcrypto_stub.c).

   The stand-in: a store counts the certificates loaded into it. X509_STORE_set_default_paths loads libcrypto's
   default places, $SSL_CERT_FILE else STUB_DEFAULT_FILE and $SSL_CERT_DIR else STUB_DEFAULT_DIR, and a file that
   is not empty counts as one certificate; a directory adds none. X509_STORE_load_locations, which the wraps call
   on the distributions' places, loads nothing: whatever the machine running the suite holds, no known place has a
   bundle. X509_verify_cert succeeds where the store holds a certificate, else fails for want of an issuer.

   `trust-roots verify` sets the default paths and verifies twice; `trust-roots never` sets them and never
   verifies. Each prints `loads <n>` on stdout, the calls of X509_STORE_load_locations. */
#include <stdio.h>
#include <string.h>

typedef struct x509_store_st X509_STORE;
typedef struct x509_store_ctx_st X509_STORE_CTX;
struct x509_store_st {
    int certificates;
};
struct x509_store_ctx_st {
    struct x509_store_st *store;
    int error;
};

int X509_STORE_set_default_paths(X509_STORE *store);
int X509_verify_cert(X509_STORE_CTX *context);
extern int stub_loads;

int main(int argc, char **argv) {
    if (argc != 2 || (strcmp(argv[1], "verify") != 0 && strcmp(argv[1], "never") != 0)) {
        fprintf(stderr, "usage: trust-roots verify|never\n");
        return 2;
    }
    X509_STORE store = {0};
    X509_STORE_set_default_paths(&store);
    if (strcmp(argv[1], "verify") == 0) {
        X509_STORE_CTX context = {&store, 0};
        X509_verify_cert(&context);
        X509_verify_cert(&context);
    }
    printf("loads %d\n", stub_loads);
    return 0;
}
