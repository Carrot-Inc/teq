// The byte streams of `java.io`, `StringBuffer` and `Matcher.region` of the platform layer.
package javaiostreams

import java.io.{ByteArrayInputStream, ByteArrayOutputStream, FilterInputStream, IOException, InputStream, OutputStream}
import java.util.regex.Pattern

class Limited(in: InputStream, limit: Int) extends FilterInputStream(in):
  private var left = limit
  override def read(): Int =
    if left <= 0 then throw new IOException("limit reached")
    left -= 1
    super.read()

object Main:
  def transfer(is: InputStream, os: OutputStream): Unit =
    var c = is.read()
    while c != -1 do
      os.write(c)
      c = is.read()

  def main(args: Array[String]): Unit =
    val out = new ByteArrayOutputStream()
    out.write(72)
    out.write("i there".getBytes("UTF-8"))
    out.write(Array[Byte](33, 33, 33), 1, 2)
    println(s"${out.size()} ${out.toString} ${out.toByteArray.length}")
    val in = new ByteArrayInputStream(out.toByteArray)
    val buf = new Array[Byte](4)
    val n = in.read(buf)
    println(s"$n ${new String(buf, 0, n, "UTF-8")} ${in.available()} ${in.read()} ${in.skip(2)} ${new String(in.readAllBytes(), "UTF-8")} ${in.read()}")
    val copy = new ByteArrayOutputStream()
    transfer(new ByteArrayInputStream("abc".getBytes("UTF-8")), copy)
    println(copy.toString("UTF-8") + " " + new ByteArrayInputStream(Array[Byte](1, 2, 3, 4), 1, 2).readAllBytes().toList)
    val lim = new Limited(new ByteArrayInputStream("xyz".getBytes("UTF-8")), 2)
    println(s"${lim.read()} ${lim.read()}")
    try lim.read()
    catch case e: IOException => println("IOException: " + e.getMessage)
    val sb = new StringBuffer(16)
    sb.append("ab").append('c').append(1).append(2.5).insert(0, "[").append("]")
    println(s"${sb.toString} ${sb.length} ${sb.charAt(1)} ${sb.indexOf("c")} ${sb.reverse().toString}")
    val m = Pattern.compile("a+").matcher("xxaaay")
    m.region(2, 5)
    println(s"${m.matches()} ${m.start()} ${m.end()} ${m.regionStart()} ${m.regionEnd()}")
    m.region(1, 6)
    println(s"${m.find()} ${m.start()} ${m.end()} ${m.lookingAt()}")
    val p = Pattern.compile(";\\s*([a-z]+)=([a-z0-9-]+)").matcher("text/plain; charset=utf-8")
    p.region(10, 25)
    println(s"${p.lookingAt()} ${p.group(1)} ${p.group(2)} ${p.end()}")
