// The other block reads C.text, whose result is inferred: the worker typing it types C's body there and then, and draws
// C's proxy ahead of B's if the first file's worker has not typed B yet; before the fork, with the quoted block, always.
package p {
  object Use {
    def text: String = defs.C.text
  }
}
package q {
  import scala.quoted.*
  object Q {
    def code(using Quotes): Expr[Int] = '{ 1 }
  }
}
