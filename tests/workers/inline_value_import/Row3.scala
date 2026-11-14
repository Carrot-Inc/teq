package crates

object Row3:
  def pull0(): String = Kit.open("R3P0")
  def pull1(): String = Kit.openGiven(new Crate("R3P1"))
  def pull2(): String = Kit.open("R3P2")
  def pull3(): String = Kit.openGiven(new Crate("R3P3"))
  def pull4(): String = Kit.open("R3P4")
  def pull5(): String = Kit.openGiven(new Crate("R3P5"))
  def pull6(): String = Kit.open("R3P6")
  def pull7(): String = Kit.openGiven(new Crate("R3P7"))
  def every(): String = pull0() + "," + pull1() + "," + pull2() + "," + pull3() + "," + pull4() + "," + pull5() + "," + pull6() + "," + pull7()
