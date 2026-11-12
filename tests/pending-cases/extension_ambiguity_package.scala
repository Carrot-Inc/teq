// A top-level given is located by its package's full name (`refStr`,
// `showLocated`) in the view search's ambiguity, the qualifier's mismatch
// (Typer.scala:4346-4353): scalac 3.8.4 "object a in package q.r".
// Queued: push 6 (typeMismatch whole, its expected side a prototype).
package q.r

trait Ops:
  extension (l: List[Int]) def pick: String
given a: Ops with
  extension (l: List[Int]) def pick: String = "a"
given b: Ops with
  extension (l: List[Int]) def pick: String = "b"

object Main:
  def main(args: Array[String]): Unit = println(List(1).pick)
