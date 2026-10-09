// The names of a named tuple are part of its type: an order or a name that differs is a
// mismatch, and the fields of the plain tuple are not members, as under scalac.
object Main:
  type Ping = (seq: Int, sentAt: String)
  def main(args: Array[String]): Unit =
    val p: Ping = (seq = 1, sentAt = "t")
    val swapped: Ping = (sentAt = "t", seq = 1)
    val other: (a: Int, b: String) = p
    println(p._1)
// expect: type mismatch: found (sentAt: String, seq: Int), required (seq: Int, sentAt: String)
// expect: type mismatch: found (seq: Int, sentAt: String), required (a: Int, b: String)
// expect: value _1 is not a member of (seq: Int, sentAt: String)
