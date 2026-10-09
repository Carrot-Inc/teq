package scala.scalajs.js

object Dynamic:
  object literal:
    @js("$jsObj(...$1)") def apply(fields: (String, Any)*): Any
    @js("({[$1]: $2})") def single(key: String, value: Any): Any

  @js("globalThis[$1]") def global(name: String): Any

object JSON:
  @js("JSON.stringify($1)") def stringify(value: Any): String

class Wrapper(val inner: Any):
  @js("$0.inner[$1]") def apply(key: String): Any
