package ega

trait Named[E]:
  def name(e: E): String

object Named:
  def derived[E]: Named[E] = new Named[E]:
    def name(e: E): String = e.toString.toLowerCase

trait Enc[A]:
  def enc(a: A): String

object Codecs:
  given namedEnc: [T: Named] => Enc[T] = new Enc[T]:
    def enc(a: T): String = "enc " + summon[Named[T]].name(a)
  given setEnc: [T: Enc] => Enc[Set[T]] = new Enc[Set[T]]:
    def enc(a: Set[T]): String = a.toList.map(summon[Enc[T]].enc).mkString("{", ", ", "}")

object Utils:
  export ega.Codecs.{namedEnc, setEnc}

object Prelude:
  export ega.Codecs.{*, given}

case class Tag(name: String)

object Shapes:
  case class Box(w: Int, h: Int = 1)
  def area(b: Box, scale: Int = 1, unit: String = "u"): String = s"${b.w * b.h * scale}$unit"
  object Defaults:
    val unit = "cm"

object Everything:
  export ega.Shapes.*
