// An ascription in an inline body, of an `inline` parameter and of a plain one: the call through it names the
// ascribed class whatever the argument's (tests/support/hidden_owner/inlineabi's package-private Leaf), as
// dotty's inliner keeps the `Typed` tree and its erasure its target, so that a client built against the
// library's first version runs on its second, where Mid declares f.
package inlinelib

import inlineabi.*

object Lib:
  inline def byInline(inline x: Mid): Int = (x: Mid).f()
  inline def byValue(x: Mid): Int = (x: Mid).f()
