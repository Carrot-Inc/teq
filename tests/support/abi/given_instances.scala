// The given classes of a class, a trait and an enum instance: scalac's class `Owner$name` (no suffix of
// teq's), its constructor taking the enclosing instance first and holding it in `$outer`, with the
// accessor `<owner>$name$$$outer`; the owner's def `name(params)` answering a new one, a trait's a
// default method with its static `name$`, and a mixing object's static forwarder. A given object of
// an instance is scalac's inner module class `Owner$name$`, which its getter answers, as an inner
// object's. The object's mixin forwarder `Loud$.sized` is not listed: teq writes none, its call
// reaching the default method.
// abi: classes Base#sized Base#sized$ Loud#sized Scale#scaled Base$sized#<init> Scale$scaled#<init> Base$sized#$outer+final Base$given_Show_Int$#$outer+final Scale$scaled#$outer+final Scale$given_Show_Boolean$#$outer+final Level$leveled#$outer+final Level$given_Show_Long$#$outer+final givensi$Base$sized$$$outer givensi$Scale$scaled$$$outer show Base#given_Show_Int Base#given_Show_Int$ Loud$#given_Show_Int Scale#given_Show_Boolean Base$given_Show_Int$#<init> Scale$given_Show_Boolean$#<init> givensi$Base$given_Show_Int$$$$outer givensi$Scale$given_Show_Boolean$$$$outer Level#leveled Level#given_Show_Long Level$leveled#<init> Level$given_Show_Long$#<init> givensi$Level$leveled$$$outer givensi$Level$given_Show_Long$$$$outer
package givensi
trait Show[A] { def show(a: A): String }
trait Base:
  def tag: String = "base"
  given sized(using n: Int): Show[String] with
    def show(a: String): String = tag + a.take(n)
  given Show[Int] with
    def show(a: Int): String = tag + a
class Scale(val factor: Int):
  given scaled(using off: Int): Show[Long] with
    def show(a: Long): String = (a * factor + off).toString
  given Show[Boolean] with
    def show(a: Boolean): String = if a then factor.toString else "-"
object Loud extends Base
enum Level(val base: Int):
  case Low(n: Int) extends Level(n)
  case High(n: Int) extends Level(n * 9)
  given leveled(using n: Int): Show[Int] with
    def show(a: Int): String = (a * base + n).toString
  given Show[Long] with
    def show(a: Long): String = (a + base).toString
