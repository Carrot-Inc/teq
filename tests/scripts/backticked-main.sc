// The script's object runs: its main's one parameter is backticked `using`, a name as any other
// (Scala CLI's WrapperUtils.checkSignature takes any Term.Name).
object Main {
  def main(`using`: Array[String]): Unit = println("ran")
}
