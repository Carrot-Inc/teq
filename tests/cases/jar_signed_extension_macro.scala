// jars: inlinebodies-lib
object Main:
  def main(args: Array[String]): Unit =
    println(inlinebodies.SignedExtensionMacro.first)
    println(inlinebodies.SignedExtensionMacro.second)
