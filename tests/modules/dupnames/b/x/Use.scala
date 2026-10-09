package dnb

object Use:
  def make: dna.Named = new dna.Named:
    def name = "b"

@main def run(): Unit = println(dna.Use.make.name + Use.make.name)
