// teq: --max-inlined-trees -1
// expect: 9:9: error: Maximal number of inlined trees (-1) exceeded,
// expect: 1 error found
// scalac 3.8.4's limit of the trees an outermost call inlines (`-Xmax-inlined-trees`): its counter
// takes 3.8.4's `treeSize`, 0 for every tree, so a limit of 0 or more never trips and a negative one
// refuses the first call nested in an expansion, reported at the outermost call.
inline def x: Int = y
inline def y: Int = 1
val z = x
