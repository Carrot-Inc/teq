// The class mixing in tests/support/hidden_owner/privateclone's trait, built by teq whole and split and by scalac.
package privatecloneuse

class E extends privateclone.G

object Main:
  def main(args: Array[String]): Unit = println(new E().f())
