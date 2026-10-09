class Account(val id: String, balance: Int):
  private val fee = 1
  private def helper = "account-helper"
  def total = balance - fee
  def show = s"$id $total $helper"

class Savings(id: String, balance: Int, rate: Int) extends Account(id, balance * 2):
  private val fee = 10
  private def helper = "savings-helper"
  def withRate = balance * rate - fee
  def show2 = s"$id $balance $helper"

class Premium(balance: Int) extends Savings("premium", balance + 1, 3):
  val fee = 100
  def helper = "premium-helper"
  def show3 = s"$id $balance $fee $helper"

@main def run() =
  val s = Savings("s1", 100, 2)
  println(s.show)
  println(s.show2)
  println(s.withRate)
  val p = Premium(7)
  println(p.show)
  println(p.show2)
  println(p.show3)
  println(p.withRate)
  println(p.fee)
