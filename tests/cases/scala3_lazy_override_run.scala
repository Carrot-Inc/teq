// Adapted from scala3 tests/run/lazy-override-run.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
class A {
  lazy val x: Int = { print("/*A.x*/"); 2 }
  lazy val y: Int = { print("/*A.y*/"); 2 }
  lazy val z: Int = { print("/*A.z*/"); 2 }
}

class B extends A {
  override lazy val x: Int = { print("/*B.x*/"); 3 }
  override lazy val y: Int = { print("/*B.y*/"); 3 }
  override lazy val z: Int = { print("/*B.z/"); 3 }
}




object Test {
  def main(args: Array[String]): Unit = {
 val a = new A
 print("a.x=")
 println(a.x)

 val b = new B
 print("b.x=")
 println(b.x)
 print("b.z=")
 println(b.z)
  }
}
