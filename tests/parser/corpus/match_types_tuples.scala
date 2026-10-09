// The match types of `Tuple` and the tuple members typed by them: `head`, `tail`, `apply`,
// `size`, `++`, `zip` and `toList`, including the derivation idiom of walking a tuple with an
// inline match on `h *: t`.
import scala.compiletime.{constValue, constValueTuple, erasedValue}

case class Row(id: Int, name: String, active: Boolean)

object Main:
  inline def sizeOf[T <: Tuple]: Int = constValue[Tuple.Size[T]]

  inline def sumInts[T <: Tuple](t: T): Int =
    inline t match
      case EmptyTuple => 0
      case tt: (h *: rest) =>
        inline tt.head match
          case i: Int => i + sumInts(tt.tail)
          case _ => sumInts(tt.tail)

  inline def typeNames[T <: Tuple]: List[String] =
    inline erasedValue[T] match
      case _: EmptyTuple => Nil
      case _: (Int *: t) => "Int" :: typeNames[t]
      case _: (String *: t) => "String" :: typeNames[t]
      case _: (Boolean *: t) => "Boolean" :: typeNames[t]
      case _: (_ *: t) => "other" :: typeNames[t]

  def main(args: Array[String]): Unit =
    val t = (1, "a", true)
    val h: Tuple.Head[(Int, String, Boolean)] = t.head
    val tl: Tuple.Tail[(Int, String, Boolean)] = t.tail
    val second: Tuple.Elem[(Int, String, Boolean), 1] = t(1)
    val n: Tuple.Size[(Int, String, Boolean)] = t.size
    val cc: Tuple.Concat[(Int, String, Boolean), (Double, Long)] = t ++ (2.5, 3L)
    val z: Tuple.Zip[(Int, String, Boolean), (Double, Long)] = t.zip((1.5, 2L))
    val m: Tuple.Map[(Int, String), Option] = (Some(1), Some("x"))
    val im: Tuple.InverseMap[(Option[Int], Option[String]), Option] = (1, "y")
    val u: Tuple.Union[(Int, String)] = "s"
    val u2: Tuple.Union[(Int, String)] = 2
    val folded: Tuple.Fold[(Int, String), Boolean, [x, y] =>> (x, y)] = (1, ("s", true))
    val empty: Tuple.Size[EmptyTuple] = 0
    println(h)
    println(tl)
    println(second)
    println(n)
    println(cc)
    println(z)
    println(m)
    println(im)
    println(u)
    println(u2)
    println(folded)
    println(empty)
    println(sizeOf[(Int, String, Boolean, Double)])
    println(sumInts((1, "two", 3, 4.0, 5)))
    println(typeNames[(Int, String, Boolean, Double)])
    println((1, 2).zip(("a", "b", "c")))
    println(EmptyTuple.size)
    println((("x", 1) ++ EmptyTuple) ++ (true, 2.5))
    println((1, "two", true).toList)
    val mixed: List[Int | String] = (1, "s").toList
    println(mixed)
    println(EmptyTuple.toList)
    println(constValueTuple[("a", "b", "c")].toList)
    println(constValueTuple[("a", "b", "c")].toList.mkString("|"))
