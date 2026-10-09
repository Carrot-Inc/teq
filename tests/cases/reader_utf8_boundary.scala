//> using platform jvm
// `InputStreamReader` decides a UTF-8 sequence the input ends inside as the JDK's decoder does
// (`UTF_8.Decoder.decodeArrayLoop` under `StreamDecoder`): one already malformed is replaced at once
// (a lead c0, c1, f5 or ff, a continuation alone, e0 80, f0 8f, f4 90, f0 90 41), only one that
// more bytes can still complete waits for them, and at the end of the input a prefix is one
// replacement. Over a socket (the interpreter's stream under the std's reader, a 250 ms timeout
// showing any wait) and over a stream of the program's that refuses to be read past its pieces; the
// native path is `reader_utf8_pipe`'s. The second review's `ReaderBoundary`.
import java.io.{IOException, InputStream, InputStreamReader}
import java.net.{ServerSocket, Socket}

object Main:
  val malformed: List[List[Int]] =
    List(List(0xc0), List(0xc1), List(0xf5), List(0xff), List(0x80), List(0xe0, 0x80), List(0xf0, 0x8f), List(0xf4, 0x90), List(0xf0, 0x90, 0x41), List(0xed, 0xa0, 0x80))

  def bytes(b: List[Int]): Array[Byte] = b.map(_.toByte).toArray
  def hex(b: List[Int]): String = b.map(x => f"$x%02x").mkString(" ")

  // The characters the reader gives until it would wait (the socket's timeout) or the end.
  def socketRead(pieces: List[List[Int]]): String =
    val server = new ServerSocket(0)
    val client = new Socket("127.0.0.1", server.getLocalPort)
    val peer = server.accept()
    client.setSoTimeout(250)
    val reader = new InputStreamReader(client.getInputStream())
    pieces.foreach(p => peer.getOutputStream().write(bytes(p)))
    val out = new StringBuilder
    var done = false
    while !done do
      try
        val c = reader.read()
        out.append(c).append(' ')
        if c < 0 then done = true
      catch
        case e: java.net.SocketTimeoutException =>
          out.append("then waits")
          done = true
    client.close()
    peer.close()
    server.close()
    out.toString.trim

  // A stream that gives its pieces one read at a time and then, unless `ends`, refuses to be read.
  class Pieces(pieces: List[List[Int]], ends: Boolean) extends InputStream:
    private var left = pieces
    def read(): Int = throw new IOException("one byte at a time")
    override def read(b: Array[Byte], off: Int, len: Int): Int = left match
      case p :: rest =>
        left = rest
        System.arraycopy(bytes(p), 0, b, off, p.length)
        p.length
      case Nil => if ends then -1 else throw new IOException("would wait")

  def piecesRead(pieces: List[List[Int]], ends: Boolean): String =
    val reader = new InputStreamReader(new Pieces(pieces, ends), "UTF-8")
    val out = new StringBuilder
    var done = false
    while !done do
      try
        val c = reader.read()
        out.append(c).append(' ')
        if c < 0 then done = true
      catch
        case e: IOException =>
          out.append("then ").append(e.getMessage)
          done = true
    out.toString.trim

  def main(args: Array[String]): Unit =
    for m <- malformed do println(s"socket ${hex(m)}: ${socketRead(List(m))}")
    println(s"socket e2 82, ac: ${socketRead(List(List(0x41, 0xe2, 0x82), List(0xac)))}")
    println(s"socket e2 82: ${socketRead(List(List(0xe2, 0x82)))}")
    for m <- malformed do println(s"pieces ${hex(m)}: ${piecesRead(List(m), false)}")
    println(s"pieces 41 e2 82, ac: ${piecesRead(List(List(0x41, 0xe2, 0x82), List(0xac)), false)}")
    println(s"pieces f0 9f, 98, 80: ${piecesRead(List(List(0xf0, 0x9f), List(0x98), List(0x80)), false)}")
    println(s"pieces e2 82 at the end: ${piecesRead(List(List(0xe2, 0x82)), true)}")
    println(s"pieces f0 9f 98 at the end: ${piecesRead(List(List(0xf0, 0x9f, 0x98)), true)}")
