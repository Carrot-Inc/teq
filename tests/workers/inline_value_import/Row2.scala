package crates

object Row2:
  def pull0(): String = Kit.open("R2P0")
  def pull1(): String = Kit.openGiven(new Crate("R2P1"))
  def pull2(): String = Kit.open("R2P2")
  def pull3(): String = Kit.openGiven(new Crate("R2P3"))
  def pull4(): String = Kit.open("R2P4")
  def pull5(): String = Kit.openGiven(new Crate("R2P5"))
  def pull6(): String = Kit.open("R2P6")
  def pull7(): String = Kit.openGiven(new Crate("R2P7"))
  def every(): String = pull0() + "," + pull1() + "," + pull2() + "," + pull3() + "," + pull4() + "," + pull5() + "," + pull6() + "," + pull7()
