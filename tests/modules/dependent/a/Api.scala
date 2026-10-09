package dpa

trait Key:
  type Value
  def default: Value

object Keys:
  object Name extends Key:
    type Value = String
    def default: String = "none"
  object Count extends Key:
    type Value = Int
    def default: Int = 0

def valueOf(k: Key): k.Value = k.default
