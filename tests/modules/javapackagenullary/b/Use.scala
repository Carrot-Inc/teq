// `f()` on the jar's parameterless `f` is refused, as scalac refuses it (E050).
package jpnb

val x = new java.review.C().f()
