#implementation

> Byte encodings of the store's types for key-value backends. Source: `codec.rs`.
>
> Sources: [[canonical]]. code: `codec.rs`

Each type is encoded as a sequence of tagged fields, so values are self-describing and can be read back field by field. Used by [[redb_store]], and later by LMDB. Decoding errors become `StoreError::Codec`.
