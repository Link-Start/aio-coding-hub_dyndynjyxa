type PropertyKeyMap<TValue extends object> = Partial<Record<keyof TValue, PropertyKey>>;

export type Override<TValue, TOverrides> = Omit<TValue, keyof TOverrides> & TOverrides;

export type NullableGeneratedKeys<TValue extends object> = {
  [TKey in keyof TValue]-?: null extends TValue[TKey] ? TKey : never;
}[keyof TValue];

export type NonNullableGeneratedKeys<TValue extends object> = Exclude<
  keyof TValue,
  NullableGeneratedKeys<TValue>
>;

export type OptionalNullableGeneratedFields<TValue extends object> = Pick<
  TValue,
  NonNullableGeneratedKeys<TValue>
> &
  Partial<Pick<TValue, NullableGeneratedKeys<TValue>>>;

export type RemapGeneratedKeys<TValue extends object, TMap extends PropertyKeyMap<TValue>> = {
  [TKey in keyof TValue as TKey extends keyof TMap ? TMap[TKey] & PropertyKey : TKey]: TValue[TKey];
};

export function narrowGeneratedStringUnion<const TAllowed extends readonly string[]>(
  value: string,
  allowed: TAllowed,
  label: string
): TAllowed[number] {
  if ((allowed as readonly string[]).includes(value)) {
    return value as TAllowed[number];
  }

  throw new Error(`IPC_INVALID_LITERAL: ${label}=${value}`);
}
