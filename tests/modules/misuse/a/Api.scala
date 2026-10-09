package mua

class Account(val owner: String):
  def deposit(amount: Int): Account = this
  private def secret: String = "s"
