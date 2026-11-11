class Outer(val value: Int):
  trait Reader:
    def read(): Int = value
  inline def twice(reader: Reader): Int = reader.read() + reader.read()
  def run(): Int = twice(new Reader {})
object Main:
  def main(args: Array[String]): Unit =
    println(new Outer(7).run())
    println(new Outer(11).run())
