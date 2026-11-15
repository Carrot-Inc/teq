package eioa

// An object nested in a trait, reached downstream through a wildcard export of an object that mixes the trait
// in: the name is the export's forwarder in a build over the upstream's products as in a whole build.
trait Tints:
  object shade:
    def value: Int = 7
    var seen: Int = 0

object AllTints extends Tints
