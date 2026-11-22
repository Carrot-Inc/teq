// The diamond of traits of the native link's tests (tests/native-link/diamond/README): A0, B0 extends A0, then
// A(n) and B(n) each extending A(n-1) with B(n-1); Leaf<n> extends A(n) with B(n) and Main<n> prints its value.
package diamond

trait A0 { def value: Int = 7 }
trait B0 extends A0
trait A1 extends A0 with B0
trait B1 extends A0 with B0
trait A2 extends A1 with B1
trait B2 extends A1 with B1
trait A3 extends A2 with B2
trait B3 extends A2 with B2
trait A4 extends A3 with B3
trait B4 extends A3 with B3
trait A5 extends A4 with B4
trait B5 extends A4 with B4
trait A6 extends A5 with B5
trait B6 extends A5 with B5
trait A7 extends A6 with B6
trait B7 extends A6 with B6
trait A8 extends A7 with B7
trait B8 extends A7 with B7
trait A9 extends A8 with B8
trait B9 extends A8 with B8
trait A10 extends A9 with B9
trait B10 extends A9 with B9
trait A11 extends A10 with B10
trait B11 extends A10 with B10
trait A12 extends A11 with B11
trait B12 extends A11 with B11
trait A13 extends A12 with B12
trait B13 extends A12 with B12
trait A14 extends A13 with B13
trait B14 extends A13 with B13
trait A15 extends A14 with B14
trait B15 extends A14 with B14
trait A16 extends A15 with B15
trait B16 extends A15 with B15
trait A17 extends A16 with B16
trait B17 extends A16 with B16
trait A18 extends A17 with B17
trait B18 extends A17 with B17
trait A19 extends A18 with B18
trait B19 extends A18 with B18
trait A20 extends A19 with B19
trait B20 extends A19 with B19
trait A21 extends A20 with B20
trait B21 extends A20 with B20
trait A22 extends A21 with B21
trait B22 extends A21 with B21
trait A23 extends A22 with B22
trait B23 extends A22 with B22
trait A24 extends A23 with B23
trait B24 extends A23 with B23
trait A25 extends A24 with B24
trait B25 extends A24 with B24
trait A26 extends A25 with B25
trait B26 extends A25 with B25
trait A27 extends A26 with B26
trait B27 extends A26 with B26
trait A28 extends A27 with B27
trait B28 extends A27 with B27
trait A29 extends A28 with B28
trait B29 extends A28 with B28
trait A30 extends A29 with B29
trait B30 extends A29 with B29
trait A31 extends A30 with B30
trait B31 extends A30 with B30
trait A32 extends A31 with B31
trait B32 extends A31 with B31
trait A33 extends A32 with B32
trait B33 extends A32 with B32
trait A34 extends A33 with B33
trait B34 extends A33 with B33
trait A35 extends A34 with B34
trait B35 extends A34 with B34
trait A36 extends A35 with B35
trait B36 extends A35 with B35

class Leaf2 extends A2 with B2
object Main2 { def main(args: Array[String]): Unit = println((new Leaf2).value) }
class Leaf3 extends A3 with B3
object Main3 { def main(args: Array[String]): Unit = println((new Leaf3).value) }
class Leaf4 extends A4 with B4
object Main4 { def main(args: Array[String]): Unit = println((new Leaf4).value) }
class Leaf5 extends A5 with B5
object Main5 { def main(args: Array[String]): Unit = println((new Leaf5).value) }
class Leaf6 extends A6 with B6
object Main6 { def main(args: Array[String]): Unit = println((new Leaf6).value) }
class Leaf7 extends A7 with B7
object Main7 { def main(args: Array[String]): Unit = println((new Leaf7).value) }
class Leaf8 extends A8 with B8
object Main8 { def main(args: Array[String]): Unit = println((new Leaf8).value) }
class Leaf9 extends A9 with B9
object Main9 { def main(args: Array[String]): Unit = println((new Leaf9).value) }
class Leaf10 extends A10 with B10
object Main10 { def main(args: Array[String]): Unit = println((new Leaf10).value) }
class Leaf11 extends A11 with B11
object Main11 { def main(args: Array[String]): Unit = println((new Leaf11).value) }
class Leaf12 extends A12 with B12
object Main12 { def main(args: Array[String]): Unit = println((new Leaf12).value) }
class Leaf13 extends A13 with B13
object Main13 { def main(args: Array[String]): Unit = println((new Leaf13).value) }
class Leaf14 extends A14 with B14
object Main14 { def main(args: Array[String]): Unit = println((new Leaf14).value) }
class Leaf15 extends A15 with B15
object Main15 { def main(args: Array[String]): Unit = println((new Leaf15).value) }
class Leaf16 extends A16 with B16
object Main16 { def main(args: Array[String]): Unit = println((new Leaf16).value) }
class Leaf17 extends A17 with B17
object Main17 { def main(args: Array[String]): Unit = println((new Leaf17).value) }
class Leaf18 extends A18 with B18
object Main18 { def main(args: Array[String]): Unit = println((new Leaf18).value) }
class Leaf19 extends A19 with B19
object Main19 { def main(args: Array[String]): Unit = println((new Leaf19).value) }
class Leaf20 extends A20 with B20
object Main20 { def main(args: Array[String]): Unit = println((new Leaf20).value) }
class Leaf28 extends A28 with B28
object Main28 { def main(args: Array[String]): Unit = println((new Leaf28).value) }
class Leaf36 extends A36 with B36
object Main36 { def main(args: Array[String]): Unit = println((new Leaf36).value) }
