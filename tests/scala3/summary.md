# scala3 tests/run through teq

1824 tests

| class | tests |
|---|---|
| pass | 727 |
| wrong-output | 29 |
| run-error | 57 |
| run-timeout | 0 |
| compile-crash | 0 |
| compile-timeout | 0 |
| compile-error | 1011 |

## Compile errors by bucket

| bucket | topic | tests |
|---|---|---|
| gap | typing | 86 |
| gap | standard library | 52 |
| gap | syntax | 38 |
| gap | other | 33 |
| gap | infix call with a block or tuple argument | 28 |
| gap | named tuples and named patterns | 6 |
| gap | polymorphic function types | 4 |
| gap | Float, Byte and Short | 2 |
| gap | local and nested classes | 1 |
| left out on purpose | extends App | 369 |
| left out on purpose | inline/macro/compiletime | 133 |
| left out on purpose | Java | 103 |
| left out on purpose | reflection (getClass/classOf/ClassTag/deriving) | 55 |
| left out on purpose | Scala 2 syntax (postfix, `f _`, XML, procedure syntax) | 19 |
| left out on purpose | match / dependent / structural types | 17 |
| left out on purpose | secondary constructors, value classes, Enumeration, self types | 16 |
| left out on purpose | custom extractors (unapply) | 9 |
| left out on purpose | exceptions (throw/try) | 7 |
| left out on purpose | null | 5 |
| left out on purpose | Scala 2 implicits | 5 |
| left out on purpose | context functions | 3 |
| left out on purpose | overloading a value | 2 |
| left out on purpose | return | 1 |
| planned | tuples and functions of any arity | 11 |
| planned | anonymous classes | 6 |

## Most frequent gap messages

- 12 × `this is not a case class or enum case` (1938-2: `case ProdNonEmpty(0, _) => ()`)
- 9 × `expected an identifier, found 'super'` (NestedClasses: `override def f = AAA1.super.f;`)
- 9 × `mac is not an extractor: it has no unapply or unapplySeq for an interpolated pattern` (i8577a: `val mac"$x" = 1`)
- 7 × `type ObjectOutputStream not found` (i4446: `val out = new ObjectOutputStream(buffer)`)
- 5 × `boundary is not a member of the package` (break-opt: `import scala.util.boundary, boundary.break`)
- 4 × `a sequence wildcard can only end a sequence pattern` (i11008b: `case Foo(x, xs*) => println(s"Many $x characters $xs")`)
- 4 × `expected ':', found 'val'` (i15840: `class NatOps[N <: Nat](tracked val n: N):`)
- 4 × `value getDeclaredFields is not a member of Class[_]` (i11367: `val outerFields = getClass.getDeclaredFields.filter(_.getName.contains("$outer")`)
- 3 × `method isEmpty needs `override` modifier to override method isEmpty in trait IterableOps` (CollectionTests: `def isEmpty: Boolean`)
- 3 × `too many arguments: expected 1` (MultiArr: `val s3: Array[Array[Int]] = Array.ofDim(2,3)`)
- 3 × `only defs are allowed in an extension` (Parser: `export ops.*`)
- 3 × `missing parameter type` (fragables-extension: `x => List(IntFrag(x))`)
- 3 × `expected ']', found '='` (hmap: `TCons(HEntry[K = "name"]("foo"),`)
- 2 × `a @main method takes no parameters or a single String*` (Pouring: `@main def Test(target: Int, capacities: Int*) =`)
- 2 × `UnrolledBuffer is not a member of the package` (UnrolledBuffer: `import collection.mutable.UnrolledBuffer`)
- 2 × `expected ',' or ')', found identifier` (collections: `s = s ++ (List.range(0, iters) map (2*))`)
- 2 × `expected ',' or ')', found literal` (dedented-string-literals: `''')`)
- 2 × `value stringOf is not a member of ScalaRunTime` (enum-values: `extension [A](t: A) def show = runtime.ScalaRunTime.stringOf(t)`)
- 2 × `expected ')', found '@'` (enums-serialization-compat: `val Seq(Red @ _, Green @ _, Blue @ _, Indigo @ _) = (1 to 4).map(_ => in.readObj`)
- 2 × `expected a pattern, found identifier` (i10047: `val / = "slash"`)
- 2 × `this type cannot be tested at runtime: Matcher[A]` (i13433: `case res: Matcher[A] => Some(res)`)
- 2 × `value gc is not a member of System` (i14198: `System.gc()`)
- 2 × `Breaks is not a member of the package` (i1732: `import scala.util.control.Breaks`)
- 2 × `ambiguous given instances for Foo: foo, foo` (i18183: `assert(foo eq implicitly[Foo])`)
- 2 × `Cannot rewrite recursive call: it is not in tail position` (i20145: `def foo(i: Int): Int = {`)
- 2 × `type Product3 not found` (i9068: `case class MyClass(v1: Int, v2: Int, v3: Int, v4: Int) extends Product3[Int, Int`)
- 2 × `expected end of statement, found ','` (inline-numeric: `given BigDecimalIsFractional: BigDecimalIsConflicted, Fractional[BigDecimal]:`)
- 2 × `value Queue is not a member of mutable` (list-apply-eval: `val queue = scala.collection.mutable.Queue[String]()`)
- 2 × `cannot resolve import: NonLocalReturns not found` (nonlocal-return: `import scala.util.control.NonLocalReturns.*`)
- 2 × `Range is not a member of the package` (range: `import scala.collection.immutable.{ Range, NumericRange }`)
- 2 × `LazyList is not a member of the package` (seqlike-kmp: `import collection.immutable.LazyList`)
- 1 × `type a not found` (16405: `case x: ((a, b) => R) =>`)
- 1 × `type mismatch: found "foo", required Extract[M]` (9416: `f(new Ext, "foo")`)
- 1 × `value > is not a member of Int` (ConfManagement: `conf.rankings.takeWhile(conf.score(_) > 80).toSet`)
- 1 × `type mismatch: found Seq[Int], required IArray[Int]` (IArrayOps: `println((iarr1 ++ iarr2.toSeq).toSeq)`)
- 1 × `Factory is not a member of the package` (IArrayOps-Factory: `import scala.collection.Factory`)
- 1 × `cannot resolve import: generic not found` (IArrayOps-IsSeq: `import scala.collection.generic.{IsIterable, IsSeq}`)
- 1 × `no given instance of type BuildFrom[ArraySeq[Long], Long, IArray[Long]] was found for parameter bf` (IArrayOps-experimental-BuildFrom: `ns.lazyZip(ns.tail).map((a, b) => b - a)`)
- 1 × `type mismatch: found Int => Int, required Int` (PrivateAnd: `def callsFoo2(other: B & A): Int = other.foo`)
- 1 × `Queue is not a member of the package` (QueueTest: `import scala.collection.mutable.Queue`)

## wrong-output (29)

Course-2002-01, Course-2002-02, Course-2002-04, Course-2002-08, Course-2002-09, Course-2002-10, assert-stack, builder, final-fields, i11419, i15725, i2964, i2964b, i2964c, i2964d, i2964e, i3000b, i4947, i4947a2, i4947b, i4947c, i9939, instanceof-serializable, string-switch, t1074, t2755, t4859, t5866, typetest

## run-error (57)

enum-values-order, equality, fully-abstract-interface, fully-abstract-nat-2, future-flatmap-exec-count, hashhash, hk-alias-unification, i11174, i11174local, i11938, i1240, i12919a, i12919b, i12976, i13181, i14020, i14340, i14540-priorRun, i14540-sameRun, i14623, i15374, i15399, i1990b, i22649, i24056, i24204, i26800-trait-init-binary-compat, i2738, i3006, i3006b, i3340, i7843, i7960, i9155, lambda-null, lift-and-unlift, llift, numbereq, optimizer-array-load, prod-mirror-inacessible-ctor, string-extractor, t2378, t3088, t3235-minimal, t3273, t5857, t6090, t6154, t6534, t7336, t7396, t7912, t8601b, t8601d, traitNoInit, uninitialized-field-values, unroll-multiple

## pass (727)

1938, 19384, 19384-min, 25333, 26071, 26957, 3179, Course-2002-03, Course-2002-05, Course-2002-06, Course-2002-07, Course-2002-13, HLists, HLists-nonvariant, HelloWorld, Lazies1, Lazies2, LazyLists, LazyValsLongs, Signals1, Signals2, StackMap, Tuple-fromArray, Tuple-fromProduct, absoverride, adding-growing-set, array-addition, arraycopy, arrayview, automatic-tupling-of-function-parameters-implicit-conversion, automatic-tupling-of-function-parameters-spec, better-fors-map-elim, better-fors-map-inlined, bigDecimalCache, bigDecimalTest, block-field, boehm-berarducci, boolexprs, buffer-slice, byname-implicits-30, byname-param, byname-varargs, case-class-toString, caseClassHash, case_class_equals_fields_sort, classTags, classof-object, collections-toSelf, collective-extensions, concat-two-strings, constValueTuple, constructors, curried-using-threading, dead-code-elimination, defaultGetters, defunctionalized, delambdafy-dependent-on-param-subst, delambdafy-dependent-on-param-subst-2, delambdafy-nested-by-name, delambdafy-two-lambdas, distinct, drop-no-effects, duplicate-meth, emptypf, enum-Color, enum-List1, enum-List2, enum-List2a, enum-Option, enum-Option1, enum-Tree, enum-approx, enum-in-trait, enum-ordinal-java, enums-precise, enums-thunk, erased-inline-product-3, eta-extension, evaluation-order, exceptions, experimentalRun, export-anyval, extension-override, extension-specificity, extension-specificity2, final-var, flat-flat-flat, for-desugar-strawman, forvaleq, fully-abstract-nat-1, fully-abstract-nat-3, function-arity, functionXXL, genericTupleMembers, gestalt-optional-inline, getClassTest-valueClass, given-eta, given-triangle, given-var, groupby, hashCodeDistribution, hello, i10016, i10062, i10068a, i10068b, i10082, i10178, i10285, i10527, i10724, i10857, i10905, i1099, i11008, i11043, i11045, i1140, i1144, i11542, i11542a, i11563, i11676, i11706, i11793, i11914, i11914a, i11961, i11966, i12052, i12160, i12170, i12328b, i12597, i1263, i12729, i12759, i12759b, i12796, i12828a, i12828b, i12829, i1284, i12919, i13087a, i13131, i13146, i13146a, i13183, i13215, i13216, i13228, i13304, i13334, i13358, i13490.min, i13630, i13747, i1386, i13862, i1392a, i1392b, i1392c, i13968, i14127, i14164, i14215, i1423, i1441, i14432, i14432a, i14432b, i14432c, i14582, i14587, i14587.min, i14587.opaques, i1463, i14675, i14693, i14705, i14964, i14964b, i14970, i14983, i1503, i15101, i15190, i15302b, i15315, i15317, i1533, i15618, i1569, i1573, i15943, i15988a, i15988b, i16065, i16092, i16108, i16213, i16252, i16390, i16728, i16785, i16879, i17021, i17021.defs, i17317, i17317-b, i17332, i1748, i17549, i17555, i17584, i17584a, i1773, i17761, i1779, i17930, i1820, i1820b, i1856, i18612-a, i18612-b, i18612-c, i18612-d, i18638, i18884, i1915, i19178, i19396, i19587, i1960, i19629, i19711, i19724, i1991, i2004, i2004b, i20095, i2020, i20225, i20284, i20395, i2077, i209, i2146, i2147, i2156, i2163, i22345, i22498, i22888, i22900a, i2314, i23179, i23245b, i23245c, i23245d, i23279, i2337, i2337b, i23409, i23409b, i23444, i23477, i2360, i23693, i23776, i23875, i23901, i2396, i2396b, i2396c, i24201a, i24201b, i24357, i24420-transparent-inline-local-ref, i2456, i24573, i24673, i24826, i24877, i24926, i25000, i25000b, i25077, i2508, i25089, i25291, i25943, i26176, i26176-with-implicits, i26349, i2808, i2895a, i2916, i2939, i3018, i3200c, i3248, i3248b, i3248c, i3396, i3548, i3624, i4037, i4073b, i4073c, i4177, i4205, i4364b, i4410, i4430, i4451, i4496a, i4496b, i4557a, i4558, i4559, i4563, i4600b, i4754, i4935, i4935b, i4961, i505, i5067, i5067b, i5257, i5260, i5340, i5350, i5350b, i5350c, i5350d, i5441, i5791, i5823, i5924, i5924b, i5924c, i6664, i6664b, i6677, i6710, i6716, i6816, i689, i6891, i6902, i6987, i6987b, i6996, i7031, i7110, i7281, i7287, i7359, i7375, i7410, i7424, i744, i756, i7613, i763, i7630, i764, i7677, i768, i7798, i789, i7926, i8035, i8058, i806, i8064, i8096, i8314, i8396, i8398, i8662, i8726, i8903, i8931, i9056, i9132, i9161, i9404, i9439, i9473, i9507, i9928, implicitFunctionXXL, implicits_poly, imports, indent, indexedSeq, inline-override, inline-override-num, inline-param-semantics, innerClass, innerObject, invocation-receivers, invocationReceivers1, invocationReceivers2, irrefutable, iterator-concat, iterator-iterate-lazy, iterator3444, iterators, lambda-sam-bridge, lambda-unit, lazy-implicit-lists, lazy-implicit-nums, lazy-val-pattern, lazy-val-pattern-2, lazy-val-pattern-3, lazyVals, lazyVals_c3.0.0, lazyVals_c3.1.0, liftedTry, list_map, lists-run, literals, mapConserve, mapValues, matchable, matchnull, mixin-final-def-object-lucre, mixin-overrides, mixin-primitive-on-generic-1, mixin-primitive-on-generic-2, mixin-primitive-on-generic-3, mixin-primitive-on-generic-4, mixin-primitive-on-generic-5, mixins, multi-apply, named-patmatch, named-tuples-strictEquality, nested-object-implements-val, nestedEq, no-init-enclosing-static-objects, non-jvm-sam-non-apply, nonLocalReturns, nonlocalreturn, null-and-intersect, null-hash, nullAsInstanceOf, numeric-range, opaque-inline2, option-fold, ordered, outer-accessors, outerPatternMatch, overload_directly_applicable, overloads, partialFunctions, patch-boundary, patmat, patmat-bind-typed, patmat-option-named, priorityQueue, promotion, pure-args-byname-noinline, puzzle, puzzler54, quote-MacroOverride, quoted-sematics-1, rainwater, recursive-with-option, reduce-projections, retclosure, retsynch, returns, richWrapperEquals, rooted_stringcontext, run-bug4840, runtime, sammy_repeated, scala_enum_testing, scan, selectable-new, simpleClass, singletons, slice-strings, spec-self, stable-enum-hashcodes, static, static-module-method, statics, sticky-extmethod, streamWithFilter, string-switch-defaults-null, stringbuilder-drop, structural-contextual, summonAll, supercalls-traits, supercalls-traits-targetname, synchronized-interface, t0005, t0048, t1005, t10170, t1048, t10594, t107, t1309, t1333, t1335, t13418, t1360, t1434, t1500b, t1500c, t1672, t1697, t1987, t1987b, t1994, t2027, t2029, t2087-and-2400, t2127, t2175, t2316_run, t2333, t2418, t2446, t2526, t2594_tcpoly, t261, t266, t2754, t2867, t2876, t2958, t3004, t3026, t3038c, t3048, t3112, t3199b, t3232, t3241, t3242b, t3269, t3326, t3395, t3397, t3452, t3452c, t3452f, t3487, t3496, t3508, t3509, t3529, t3563, t3603, t363, t3645, t3651, t3699, t3702, t3714, t3760, t3761-overload-byname, t3855, t3877, t3895, t3923, t3980, t4013, t4013b, t4013c, t4054, t4062, t408, t4122, t4148, t4288, t429, t4297, t4351, t4398, t4400, t4415, t4482, t4558, t4565_1, t4577, t4582, t4592, t4601, t4660, t4697, t4709, t4753, t4770, t4827, t4827b, t4835, t493, t4954, t4996, t5037, t5105, t5158, t5162, t5284b, t5284c, t5300, t5552, t5568, t5604, t5608, t5629, t5648, t5665, t594, t5971, t601, t603, t6089, t6126, t6188, t6206, t6260, t627, t6272, t629, t6337a, t6370, t6385, t6443-by-name, t6443-varargs, t6476, t6506, t6559, t6646, t6666a, t6928-run, t6957, t6968, t7019, t704, t7126, t7278, t7406, t7407, t7407b, t7436, t7475b, t7763, t7859, t8017, t8100, t8133, t8133b, t8233, t8233-bcode, t8245, t8280, t8395, t8570, t8570a, t8601, t8607, t8738, t8893, t8893b, t8933b, t8933c, t920, t920_1, t9516, takeAndDrop, targetName, targetName-separate, toList, trailingCommas, traitInit, traitParamInit, traitParams, traitValBridge, traitValOverriddenByParamAccessor, traits-initialization, transparent-implicits, transparent-object, transparentAccess, transparentAssign, transparentByName, transparentPower, transparentPrivates, transpose, triple-quoted-expr, try-2, tryPatternMatch, tuple-accessors, tuple-cons-2, tuple-deconstruct-wildcard-sideeffect, tuple-erased, tuple-for-comprehension, tuple-match, tuple-patmat, tuple-patmat-extract, tuple-patmat-size, tuple-product, tuple-underscore-syntax, tuples-empty, typable, type-test-binding, type-test-nat, unapplyArray, unittest_iterator, unroll-inferredFinal, value-class-partial-func-depmet, valueclasses-nested-object, valueclasses-pavlov, verify-ctor, view-headoption, whitebox-inline, wildcard-vals, withIndex
