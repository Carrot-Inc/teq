package probe.pa
import scala.deriving.Mirror
import scala.compiletime.constValue
trait Enc[A]:
  def enc(a: A): String
object Enc:
  given Enc[Int] = a => a.toString
  given option[A](using e: Enc[A]): Enc[Option[A]] = o => o.map(e.enc).getOrElse("null")
  inline given auto[T](using m: Mirror.Of[T]): Enc[T] = a => constValue[m.MirroredLabel]
enum Alert:
  case Wind(speed: Int)
  case Quiet
object Main:
  def main(args: Array[String]): Unit =
    println(summon[Enc[Option[Alert]]].enc(Some(Alert.Quiet)))
    println(summon[Enc[Alert.Wind]].enc(Alert.Wind(1)))
