// java.util.UUID.nameUUIDFromBytes: the JDK's version-3 recipe (the bytes hashed by MD5 as they are,
// byte 6 `(b & 15) | 48`, byte 8 `(b & 63) | 128`, two big-endian halves) over the empty input, text,
// the lengths about MD5's block boundaries and every byte value; a null array is a NullPointerException.
package uuidnamebytes

import java.nio.charset.StandardCharsets.UTF_8
import java.util.UUID

@main def main(): Unit =
  def show(label: String, bytes: Array[Byte]): Unit =
    val u = UUID.nameUUIDFromBytes(bytes)
    println(s"$label $u v=${u.version} var=${u.variant} ${u.getMostSignificantBits} ${u.getLeastSignificantBits}")
  show("empty", new Array[Byte](0))
  show("hello", "hello".getBytes(UTF_8))
  show("text", "The quick brown fox jumps over the lazy dog".getBytes(UTF_8))
  show("unicode", "naïve – 日本".getBytes(UTF_8))
  for n <- List(1, 3, 55, 56, 57, 63, 64, 65, 119, 120, 121, 128, 1000) do
    show(s"len$n", Array.tabulate[Byte](n)(i => (i * 7 + n).toByte))
  show("every", Array.tabulate[Byte](256)(_.toByte))
  show("zeros", new Array[Byte](64))
  show("ones", Array.fill[Byte](64)(-1))
  val bytes = "same".getBytes(UTF_8)
  println(UUID.nameUUIDFromBytes(bytes) == UUID.nameUUIDFromBytes("same".getBytes(UTF_8)))
  println(bytes.toList)
  val name = UUID.nameUUIDFromBytes("x".getBytes(UTF_8))
  println(UUID.fromString(name.toString) == name)
  try println(UUID.nameUUIDFromBytes(null))
  catch case e: NullPointerException => println("NullPointerException")
