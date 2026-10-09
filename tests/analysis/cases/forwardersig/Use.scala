package fws

// A member whose signature names a type through an export's alias, used from another file.
class Use:
  def g: Int = F.value
