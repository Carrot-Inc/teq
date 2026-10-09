// A `CharSequence` value, a string or a builder, through the trait's members.
package charsequencemembers

def show(cs: CharSequence): String =
  val c = cs.charAt(1)
  s"${cs.length} ${c.toInt} ${c == 'b'} ${cs.isEmpty} ${cs.subSequence(1, 3)} ${cs.subSequence(0, 2).length}"

@main def main(): Unit =
  println(show("abcd"))
  println(show(new java.lang.StringBuilder("wxyz")))
  println(show("p" + "qr"))
  val empty: CharSequence = ""
  println(empty.isEmpty)
  println(empty.length)
