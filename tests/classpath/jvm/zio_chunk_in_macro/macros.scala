// std: scala-library
// jars: scala-library zio-jvm zio-stacktracer-jvm zio-internal-macros-jvm izumi-reflect-jvm izumi-reflect-boopickle-jvm
//> using dep dev.zio::zio:2.1.26
// zio's `Chunk` in a macro run, as the layer macro of `provide` uses it: `Singleton` overrides
// the generic alternative `toArray[A1 >: A](srcPos: Int, dest: Array[A1], destPos: Int,
// length: Int)`, which no override record names, so the interpreter finds it by signature
// rather than running `Chunk`'s own, which materialises again without end; and the JDK's
// `String.valueOf`, which the layer macro's rendering calls through `StringBuilder.append`, by
// the overload selected.
import scala.quoted.*
import zio.Chunk

object Probe:
  inline def run: String = ${ impl }
  def impl(using Quotes): Expr[String] =
    val c = Chunk.single(5)
    val a = c.toArray.toList
    val m = c.materialize.toList
    val cat = (Chunk(1, 2) ++ Chunk.single(3)).toArray.toList
    val sb = new StringBuilder(capacity = 10).append("x").append(1).append('c').append(2.5).append(null: Any).result()
    val v = String.valueOf(3) + String.valueOf('d') + String.valueOf(true) + String.valueOf(Array('a', 'b', 'c')) + String.valueOf(Array('a', 'b', 'c'), 1, 2)
    // The overload the call selects decides, not the value: an array as an `Object` is not
    // its characters, a `null` `char[]` throws.
    val asObject = String.valueOf(Array('a', 'b'): Object) != "ab"
    val nullChars =
      try String.valueOf(null: Array[Char])
      catch case _: NullPointerException => "npe"
    Expr(s"$a $m $cat $sb $v $asObject $nullChars ${String.valueOf(null: Object)}")
