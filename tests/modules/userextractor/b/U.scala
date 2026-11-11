// A downstream of a/, which runs a/'s bodies from its products in the interpreter and in JavaScript. A user
// extractor's pattern pickles `UNAPPLY` of the extractor's reference (`fun_Term`, a SELECTin of its signed name, as
// TreePickler's `pickleTree(fun)` writes it); the writer wrote the Select as a call of the extractor with no argument,
// failed its arity ("a call of unapply with 0 arguments for 1 parameters") and withheld the body of `words` and `opt`
// from every products build, a regression of the type-representation push (master efddb38de3 writes them). A run
// over the products stopped on the withheld body. Pinned as products_user_unapply_seq_withheld.
package uxb

import uxa.Bodies.X

@main def run(): Unit =
  println(X.pair("a b"))
  println(X.words("x y"))
  println(X.opt("o"))
  println(X.opt(""))
