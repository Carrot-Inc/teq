package tia

// Traits whose initialisers run in the classes mixing them in, upstream and downstream.
trait Logging:
  print("[logging] ")
  val prefix: String = "log:"

trait Counted:
  var count = 0
  count += 1

class Service extends Logging with Counted:
  print("[service] ")
  def describe = prefix + count
