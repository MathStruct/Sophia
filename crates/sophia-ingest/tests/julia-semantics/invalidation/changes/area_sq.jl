# A more specific method: a call on `Sq`, `Shape` or `Any` may now reach it.
Inval.area(s::Inval.Sq) = 2.0
