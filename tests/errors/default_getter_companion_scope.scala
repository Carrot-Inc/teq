// A constructor's default getters are members of the companion object (`Desugar.classDef`'s
// `companionDefs`), typed in its scope: a default cannot name the class's own members
// (`Not found: x`), as it could in the class's definition scope.
// expect: 6:25: error: not found: x
// expect: 1 error found
class Test(val n: Int = x) { val x: Int = 42 }
