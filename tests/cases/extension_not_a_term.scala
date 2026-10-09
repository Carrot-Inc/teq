// An extension method of a package is found through its receiver, or called by its name with
// the receiver as its first argument; a bare name inside an enum's companion is the companion's
// member of that name.
package extensionnotaterm

extension (s: String) def values: Int = s.length

enum Color:
  case Red, Green

object Color:
  def names: List[String] = values.toList.map(_.toString)

@main def main(): Unit =
  println(Color.names)
  println("abc".values)
  println(values("abcd"))
