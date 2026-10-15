package mwa

// An anonymous class's method whose body the TASTy writer withholds (`IndexedSeq.range`), which a
// downstream macro reaches at each of its expansions.
trait Job:
  def run(): Int
  def name: String

object Jobs:
  def make: Job = new Job:
    def run(): Int = IndexedSeq.range(0, 3).length
    def name: String = "job"
