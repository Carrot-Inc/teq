object Names { val `given` = 1 }
object Use { import Names.{`given`, *}; val n = `given` }
