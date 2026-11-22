// Member types of a class file twelve levels deep and ten, each read where it is asked, whatever the depth: by its full
// path, through an import, a generic one applied, one a Scala class extends, inner ones as types, and in a Java
// source's signature.
import J.N1.N2.N3.N4.N5.N6.N7.N8.N9.N10.N11.N12
class S extends J.N1.N2.N3.N4.N5.N6.N7.N8.N9.N10.N11.N12
object Main {
  def inner(x: Option[J#I1#I2#I3#I4#I5#I6#I7#I8#I9#I10]): Int = x.map(_.value()).getOrElse(7)
  def main(args: Array[String]): Unit =
    println(s"${new J.N1.N2.N3.N4.N5.N6.N7.N8.N9.N10.N11.N12().value()} ${new N12().value()} ${new J.G1.G2.G3.G4.G5.G6.G7.G8.G9.G10[String]().value()} ${new S().value()} ${inner(None)} ${Use.make().value()}")
}
