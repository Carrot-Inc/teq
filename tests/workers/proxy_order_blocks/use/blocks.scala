// Two package blocks of one source, the second with quoted code: dotty types the source in order, so the first
// block's expansion draws m$proxy1 and the second's m$proxy2, as one worker's walk does.
package p {
  object First:
    def arg(): Int = 1
    def text: String = lib.Names.bound(arg())
}
package q {
  import scala.quoted.*
  object Second:
    def code(using Quotes): Expr[Int] = '{ 1 }
    def arg(): Int = 2
    def text: String = lib.Names.bound(arg())
}
