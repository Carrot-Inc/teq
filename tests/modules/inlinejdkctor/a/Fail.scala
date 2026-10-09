package ija

// An inline body constructing a JDK exception with its message alone, which the std models as
// a class whose cause has a default: a downstream's expansion of the product's body passes the
// default (`Exception(String, Throwable)`), the whole build's calls `Exception(String)`.
object Fail:
  inline def fail(msg: String): Nothing = throw new Exception(s"failed: $msg")
