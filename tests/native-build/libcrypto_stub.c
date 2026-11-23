/* The stand-in for libcrypto of the native-build suite's check of the trust roots' wraps (trust_roots.c, which has
   the check's main and says what the stand-in does): the functions src/native/static_musl.c's wraps call, in an
   object of their own, since a linker's -wrap replaces a call to a function from another object alone. */
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

struct x509_store_st {
    int certificates;
};
struct x509_store_ctx_st {
    struct x509_store_st *store;
    int error;
};
struct stack_st {
    int count;
};

typedef struct x509_store_st X509_STORE;
typedef struct x509_store_ctx_st X509_STORE_CTX;
typedef struct stack_st OPENSSL_STACK;

int X509_STORE_set_default_paths(X509_STORE *store);
int X509_verify_cert(X509_STORE_CTX *context);

int stub_loads;
static OPENSSL_STACK objects;

const char *X509_get_default_cert_file_env(void) { return "SSL_CERT_FILE"; }
const char *X509_get_default_cert_dir_env(void) { return "SSL_CERT_DIR"; }

static const char *stub_default(const char *variable) {
    const char *value = getenv(variable);
    return value != NULL ? value : "";
}

const char *X509_get_default_cert_file(void) { return stub_default("STUB_DEFAULT_FILE"); }
const char *X509_get_default_cert_dir(void) { return stub_default("STUB_DEFAULT_DIR"); }

static const char *place(const char *variable, const char *otherwise) {
    const char *value = getenv(variable);
    return value != NULL ? value : otherwise;
}

/* Wrapped: the "real" function the wrap calls. */
int X509_STORE_set_default_paths(X509_STORE *store) {
    struct stat file;
    if (stat(place("SSL_CERT_FILE", X509_get_default_cert_file()), &file) == 0 && S_ISREG(file.st_mode) && file.st_size > 0)
        store->certificates++;
    return 1;
}

int X509_STORE_load_locations(X509_STORE *store, const char *file, const char *directory) {
    (void)store, (void)file, (void)directory;
    stub_loads++;
    return 0;
}

int X509_STORE_lock(X509_STORE *store) { return store != NULL; }
int X509_STORE_unlock(X509_STORE *store) { return store != NULL; }

OPENSSL_STACK *X509_STORE_get0_objects(const X509_STORE *store) {
    objects.count = store->certificates;
    return &objects;
}

int OPENSSL_sk_num(const OPENSSL_STACK *stack) { return stack->count; }
X509_STORE *X509_STORE_CTX_get0_store(const X509_STORE_CTX *context) { return context->store; }
int X509_STORE_CTX_get_error(const X509_STORE_CTX *context) { return context->error; }
void ERR_clear_error(void) {}

/* Wrapped: fails with X509_V_ERR_UNABLE_TO_GET_ISSUER_CERT_LOCALLY where the store is empty. */
int X509_verify_cert(X509_STORE_CTX *context) {
    if (context->store->certificates > 0)
        return 1;
    context->error = 20;
    return 0;
}
