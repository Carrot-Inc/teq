// `inline match` over a type (`erasedValue[T]`), `inline if` over an inline parameter and
// `constValue`, pickled unreduced with their markers, which scalac reduces at each call.
import scala.compiletime.{constValue, erasedValue}

object Kinds:
  inline def kind[T]: String = inline erasedValue[T] match
    case _: Int => "int"
    case _: String => "string"
    case _: List[?] => "list"
    case _ => "other"
  inline def pick(inline flag: Boolean): String = inline if flag then "yes" else "no"
  inline def size[T <: Tuple]: Int = constValue[Tuple.Size[T]]

object InlineMatchType:
  def main(args: Array[String]): Unit =
    println(Kinds.kind[Int] + " " + Kinds.kind[String] + " " + Kinds.kind[List[Int]] + " " + Kinds.kind[Double])
    println(Kinds.pick(true) + " " + Kinds.pick(false))
    println(Kinds.size[(Int, String, Boolean)])
