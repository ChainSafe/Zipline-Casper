#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[derive(Default, Debug, PartialEq, Eq, SimpleSerialize)]
    struct Foo {
        a: u32,
    }

    #[derive(Default, Debug, PartialEq, Eq, SimpleSerialize)]
    struct Bar {
        a: List<u32, 128>,
    }

    #[derive(Default, Debug, PartialEq, Eq, SimpleSerialize)]
    struct BasicContainer {
        a: u32,
        d: bool,
    }

    #[derive(Default, Debug, PartialEq, Eq, SimpleSerialize)]
    struct SomeContainer {
        a: u32,
        b: bool,
        c: List<bool, 32>,
    }

    #[derive(Default, Debug, PartialEq, Eq, SimpleSerialize)]
    struct AnotherContainer {
        a: u32,
        b: bool,
        c: List<bool, 32>,
        d: Vector<bool, 4>,
        e: u8,
    }

    #[derive(Default, Debug, PartialEq, Eq, SimpleSerialize)]
    struct YetAnotherContainer {
        a: u32,
        b: bool,
        c: List<bool, 32>,
        d: Vector<bool, 4>,
        e: u8,
        f: List<u32, 32>,
    }

    #[derive(Default, Debug, PartialEq, Eq, SimpleSerialize)]
    struct VarTestStruct {
        a: u16,
        b: List<u16, 1024>,
        c: u8,
    }

    #[derive(Default, Debug, PartialEq, Eq, SimpleSerialize)]
    struct VarWithGenericTestStruct<const N: usize> {
        a: u16,
        b: List<u16, N>,
        c: u8,
    }

    #[derive(Default, Debug, PartialEq, Eq, SimpleSerialize)]
    struct TupleStruct(u8);

    #[test]
    fn encode_container() {
        let value = Foo { a: 5u32 };

        let mut buffer = vec![];
        let result = value.serialize(&mut buffer).expect("can serialize");
        assert_eq!(result, 4);
        let expected = [5u8, 0u8, 0u8, 0u8];
        assert_eq!(buffer, expected);

        let value = Bar { a: Default::default() };

        let mut buffer = vec![];
        let result = value.serialize(&mut buffer).expect("can serialize");
        assert_eq!(result, 4);
        let expected = [4u8, 0u8, 0u8, 0u8];
        assert_eq!(buffer, expected);

        let value = BasicContainer { a: 5u32, d: true };

        let mut buffer = vec![];
        let result = value.serialize(&mut buffer).expect("can serialize");
        assert_eq!(result, 5);
        let expected = [5u8, 0u8, 0u8, 0u8, 1u8];
        assert_eq!(buffer, expected);
    }

    #[test]
    fn encode_container2() {
        let value =
            SomeContainer { a: 5u32, b: true, c: List::try_from(vec![true, false]).unwrap() };

        let mut buffer = vec![];
        let result = value.serialize(&mut buffer).expect("can serialize");
        assert_eq!(result, 11);
        let expected = [5u8, 0u8, 0u8, 0u8, 1u8, 9u8, 0u8, 0u8, 0u8, 1u8, 0u8];
        assert_eq!(buffer, expected);
    }

    #[test]
    fn encode_container3() {
        let value = AnotherContainer {
            a: 5u32,
            b: true,
            c: List::try_from(vec![true, false]).unwrap(),
            d: Default::default(),
            e: 12u8,
        };

        let mut buffer = vec![];
        let result = value.serialize(&mut buffer).expect("can serialize");
        assert_eq!(result, 16);
        let expected =
            [5u8, 0u8, 0u8, 0u8, 1u8, 14u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 12u8, 1u8, 0u8];
        assert_eq!(buffer, expected);
    }

    #[test]
    fn decode_container() {
        let data = vec![5u8, 0u8, 0u8, 0u8, 1u8, 9u8, 0u8, 0u8, 0u8, 1u8, 0u8];
        let result = SomeContainer::deserialize(&data).expect("can deserialize");
        let value =
            SomeContainer { a: 5u32, b: true, c: List::try_from(vec![true, false]).unwrap() };
        assert_eq!(result, value);
    }

    #[test]
    fn roundtrip_container() {
        let value = AnotherContainer {
            a: 5u32,
            b: true,
            c: List::try_from(vec![true, false, false, false, true, true]).unwrap(),
            d: Vector::try_from(vec![true, false, false, true]).unwrap(),
            e: 24u8,
        };
        let mut buffer = vec![];
        let _ = value.serialize(&mut buffer).expect("can serialize");
        let recovered = AnotherContainer::deserialize(&buffer).expect("can decode");
        assert_eq!(value, recovered);

        let value = YetAnotherContainer {
            a: 5u32,
            b: true,
            c: List::try_from(vec![true, false, false, false, true, true]).unwrap(),
            d: Vector::try_from(vec![true, false, false, true]).unwrap(),
            e: 24u8,
            f: List::try_from(vec![234u32, 567u32]).unwrap(),
        };
        let mut buffer = vec![];
        let _ = value.serialize(&mut buffer).expect("can serialize");
        let recovered = YetAnotherContainer::deserialize(&buffer).expect("can decode");
        assert_eq!(value, recovered);
    }

    #[test]
    fn decode_container_with_extra_input() {
        let data = vec![5u8, 0u8, 7u8, 0u8, 0u8, 0u8, 5u8, 255u8];
        let result = VarTestStruct::deserialize(&data);
        assert!(result.is_err());
    }

    #[test]
    fn can_derive_struct_with_const_generics() {
        let value = VarWithGenericTestStruct {
            a: 2u16,
            b: List::<u16, 2>::try_from(vec![1u16]).unwrap(),
            c: 16u8,
        };
        let mut buffer = vec![];
        let _ = value.serialize(&mut buffer).expect("can serialize");
    }

    #[test]
    fn can_derive_tuple_struct() {
        let value = TupleStruct(22);
        let mut buffer = vec![];
        let _ = value.serialize(&mut buffer).expect("can serialize");
    }
}
