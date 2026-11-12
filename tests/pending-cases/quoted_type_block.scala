// A quoted type pattern that defines its type variable (`'[ type a; List[a] ]`): scalac
// 3.8.4 compiles it, and teq is to compile it too. The method is compiled, never called.
import scala.quoted.*
def elementKind[T](t: Type[T])(using Quotes): Int = t match
  case '[ type a; List[a] ] => 1
  case _ => 0
@main def run(): Unit = println(1)
