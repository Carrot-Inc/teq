package hedge

class Trunk { type Pith = Int; class Knot }
trait Limb extends Trunk { type Twig = Int }
trait Spray extends Limb { type Bud = Int }
class Hedge extends Spray { type Thorn = Int }
trait Loose { type Mid }
class Thicket extends Loose with Limb
