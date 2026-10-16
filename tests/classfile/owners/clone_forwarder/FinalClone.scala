// A trait's final `clone`: the class's forwarder is final as the member is (`MixinOps.mkForwarderSym`
// copies its flags), so that javac refuses a Java subclass's override as it refuses it against scalac's
// classes (FinalClient.java, tests/classfile.scala).
package finalclone

trait C:
  final override def clone(): String = "final"
class E extends C

object Main:
  def main(args: Array[String]): Unit = println((new E).clone())
