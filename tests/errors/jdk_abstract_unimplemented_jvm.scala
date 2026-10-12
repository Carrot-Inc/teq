// expect: 9:7: error: class Empty needs to be abstract, since def read(): Int in class InputStream
// expect: 10:7: error: class Quiet needs to be abstract, since def write(
// expect: 2 errors found
// teq: --target jvm
// A member the JDK declares abstract (`ACC_ABSTRACT`, dotty's `ClassfileParser` reading it `Deferred`) is implemented
// by a member of its signature alone: `InputStream.read(Array[Byte])` implements no `read()`. (scalac names the
// class's package after it, `in class InputStream in package java.io`, and a JDK parameter `x0`.)
import java.io.{InputStream, OutputStream}
class Empty extends InputStream
class Quiet extends OutputStream:
  override def write(b: Array[Byte]): Unit = ()
@main def run() = println(new Empty)
