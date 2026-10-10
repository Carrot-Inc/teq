// An upstream inline method's `inline match` evaluates its scrutinee once where no case reads it, the
// downstream's expansion typing the body again (`InlineReducer.reduceInlineMatch`, `Inliner.dropUnusedDefs`).
package imea

inline def typeOnly(inline x: Int): Int = inline x match { case _: Int => 7 }
inline def byName(x: => Int): Int = inline x match { case _: Int => 8 }
