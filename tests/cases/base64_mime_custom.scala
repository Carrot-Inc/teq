// java.util.Base64.getMimeEncoder(lineLength, lineSeparator) as the JDK has it: the separator checked
// (a base64 character or `=` refused, even with wrapping off), the length rounded down to a multiple of
// 4, the basic encoder when that is not positive, the separator between lines alone (never trailing),
// the array kept by reference through `withoutPadding`, `encode` and `encodeToString` alike.
package base64mimecustom

import java.nio.charset.StandardCharsets.UTF_8
import java.util.Base64

@main def main(): Unit =
  val lf = "\n".getBytes(UTF_8)
  val data = Array.tabulate[Byte](100)(i => (i * 37 + 11).toByte)
  val enc = Base64.getMimeEncoder(64, lf)
  for n <- List(0, 1, 2, 3, 47, 48, 49, 50, 96, 97, 100) do
    println(s"$n ${enc.encodeToString(data.take(n)).replace("\n", "|")}")
  println(Base64.getMimeEncoder(5, lf).encodeToString("AAAA".getBytes(UTF_8)).replace("\n", "|"))
  println(Base64.getMimeEncoder(5, lf).withoutPadding().encodeToString("AAAA".getBytes(UTF_8)).replace("\n", "|"))
  for len <- List(-8, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 76, 77) do
    println(s"len $len ${Base64.getMimeEncoder(len, lf).encodeToString(data.take(13)).replace("\n", "|")}")
  println(Base64.getMimeEncoder(76, "\r\n".getBytes(UTF_8)).encodeToString(data) == Base64.getMimeEncoder().encodeToString(data))
  println(Base64.getMimeEncoder(Int.MaxValue, lf).encodeToString(data) == Base64.getEncoder().encodeToString(data))
  val refused = (0 until 256).filter { b =>
    try { Base64.getMimeEncoder(4, Array(b.toByte)); false }
    catch case _: IllegalArgumentException => true
  }
  println(refused.size)
  println(refused.map(_.toChar).mkString)
  for sep <- List("A", "z", "5", "+", "/", "=", "-=") do
    try Base64.getMimeEncoder(8, sep.getBytes(UTF_8))
    catch case e: IllegalArgumentException => println(s"$sep ${e.getMessage}")
  try Base64.getMimeEncoder(0, Array('='.toByte))
  catch case e: IllegalArgumentException => println(s"off ${e.getMessage}")
  try Base64.getMimeEncoder(-4, Array('a'.toByte))
  catch case e: IllegalArgumentException => println(s"negative ${e.getMessage}")
  try Base64.getMimeEncoder(8, null)
  catch case _: NullPointerException => println("NullPointerException")
  println(Base64.getMimeEncoder(4, new Array[Byte](0)).encodeToString(data.take(10)))
  println(Base64.getMimeEncoder(4, "-_".getBytes(UTF_8)).encodeToString(data.take(10)))
  val high = Base64.getMimeEncoder(4, Array(0.toByte, -1.toByte)).encodeToString(data.take(7))
  println(high.map(_.toInt).mkString(","))
  val bytes = Base64.getMimeEncoder(4, Array(0.toByte, -1.toByte)).encode(data.take(7))
  println(bytes.toList)
  println(new String(Base64.getMimeEncoder(8, lf).encode(data.take(12)), UTF_8).replace("\n", "|"))
  println(Base64.getMimeEncoder(8, lf).encode(new Array[Byte](0)).length)
  val sep = "\n".getBytes(UTF_8)
  val shared = Base64.getMimeEncoder(4, sep)
  val unpadded = shared.withoutPadding()
  sep(0) = 'A'.toByte
  println(shared.encodeToString(data.take(8)))
  println(unpadded.encodeToString(data.take(8)))
  println(Base64.getMimeDecoder().decode(Base64.getMimeEncoder(8, lf).encodeToString(data)).toList == data.toList)
