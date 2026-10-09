// `DataOutputStream` and `DataInputStream`: the primitives big-endian, a string in modified
// UTF-8 after its length, and the end of the input an `EOFException` (Scala.js's test bridge
// frames its messages with them).
import java.io.*

@main def run(): Unit =
  val bytes = new ByteArrayOutputStream()
  val out = new DataOutputStream(bytes)
  out.writeBoolean(true)
  out.writeByte(-2)
  out.writeShort(0x1234)
  out.writeChar('é')
  out.writeInt(-123456789)
  out.writeLong(0x0123456789abcdefL)
  out.writeFloat(1.5f)
  out.writeDouble(-2.25)
  out.writeUTF("héllo ✓")
  out.write(255)
  println(out.size())
  out.close()
  val raw = bytes.toByteArray
  println(raw.take(8).map(b => b & 0xff).mkString(" "))
  val in = new DataInputStream(new ByteArrayInputStream(raw))
  println(in.readBoolean())
  println(in.readByte())
  println(in.readShort())
  println(in.readChar())
  println(in.readInt())
  println(in.readLong())
  println(in.readFloat())
  println(in.readDouble())
  println(in.readUTF())
  println(in.readUnsignedByte())
  try in.readInt() catch case _: EOFException => println("end")
