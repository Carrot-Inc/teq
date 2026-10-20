// lila: gathering/Condition.scala:37 (`def name(pt: PerfType)(using Translate) = name`)
// An overriding def without a result type takes the overridden one's type without typing its
// body, so its body may refer to an overload of the same name.
trait Condition:
  def name(n: Int): String

case class Titled(public: Boolean) extends Condition:
  def name(n: Int) = name
  def name = if public then "Public titled accounts" else "Titled accounts"

@main def run() = println(Titled(true).name(1))
