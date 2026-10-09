package sharedlib

trait Show[A]:
  def show(a: A): String

object Utils:
  def clamp(x: Int, lo: Int, hi: Int): Int = if x < lo then lo else if x > hi then hi else x

  val defaultLimit: Int = 25

  lazy val banner: String = "== utils =="

  enum Level:
    case Low, High
    case Custom(n: Int)

  opaque type UserId = Int

  object UserId:
    def apply(n: Int): UserId = n

  extension (id: UserId)
    def value: Int = id
    def next: UserId = id + 1

  extension (s: String) def shout: String = s.toUpperCase + "!"

  given Show[Int] with
    def show(a: Int): String = s"int:$a"

  given showLevel: Show[Level] with
    def show(l: Level): String = l match
      case Level.Low => "low"
      case Level.High => "high"
      case Level.Custom(n) => s"custom($n)"

  given [A](using s: Show[A]): Show[List[A]] with
    def show(as: List[A]): String = as.map(s.show).mkString("[", ",", "]")

  private def internal: Int = 1
