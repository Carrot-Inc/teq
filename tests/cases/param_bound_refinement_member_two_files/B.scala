type Selfish = AnyRef { def self: Int }
class Knot { def self: Int = 31 }
class Wide[F[X] <: AnyRef { def self: Int }] { def tag: Int = 32 }
class Fixed[X] { def self: Int = 1 }
