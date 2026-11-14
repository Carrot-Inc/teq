package crates

object Row7:
  def pull0(): String = Kit.open("R7P0")
  def pull1(): String = Kit.openGiven(new Crate("R7P1"))
  def pull2(): String = Kit.open("R7P2")
  def pull3(): String = Kit.openGiven(new Crate("R7P3"))
  def pull4(): String = Kit.open("R7P4")
  def pull5(): String = Kit.openGiven(new Crate("R7P5"))
  def pull6(): String = Kit.open("R7P6")
  def pull7(): String = Kit.openGiven(new Crate("R7P7"))
  def every(): String = pull0() + "," + pull1() + "," + pull2() + "," + pull3() + "," + pull4() + "," + pull5() + "," + pull6() + "," + pull7()
