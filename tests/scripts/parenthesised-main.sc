// The script's object runs: its main's parameter type is a parenthesised Array[String], which the parser
// reads as Array[String] (Scala CLI's WrapperUtils.checkSignature over the parsed type).
object Main {
  def main(args: (Array[String])): Unit = println("ran")
}
