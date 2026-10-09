package demandedretained

object Plain:
  def sign(x: Int) = inline if x > 0 then 1 else -1
