package taa

// An upstream trait whose abstract members nothing upstream calls: its products declare them all,
// for the downstream modules that call them.
trait Api:
  def value: Int
  def twice: Int = value * 2

trait Named[A]:
  extension (a: A) def named: String

trait Unused:
  def never(x: String): String
