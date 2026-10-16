package hidden;

// Package-private: a member it declares is reached from another package through a public
// subclass alone. Two levels above Shown.
abstract class Root {
    public final int deep() { return 11; }
}
