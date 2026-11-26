// A block of a local class and a local def that ends in the class's literal runs to that literal (its class files:
// tests/support/abi/lifted_class_literal_block.scala).
object Main {
  def run(): Class[?] = { class Local; def kept(): Int = 7; classOf[Local] }
  def main(args: Array[String]): Unit = println(run() != null)
}
