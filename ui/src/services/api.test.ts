// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import { parseJsonWithBigInt } from './api';

// A bunch of AI tests for good measure, while we're using this function to parse the response JSON -> javascript object with BigInt support.
describe('parseJsonWithBigInt', () => {
  describe('standard JSON parsing', () => {
    it('should parse simple objects without big integers', () => {
      const json = '{"name": "test", "count": 42}';
      const result = parseJsonWithBigInt<{ name: string; count: number }>(json);

      expect(result).toEqual({ name: 'test', count: 42 });
    });

    it('should parse arrays without big integers', () => {
      const json = '[1, 2, 3, 4, 5]';
      const result = parseJsonWithBigInt<number[]>(json);

      expect(result).toEqual([1, 2, 3, 4, 5]);
    });

    it('should parse nested objects', () => {
      const json = '{"outer": {"inner": {"value": 123}}}';
      const result = parseJsonWithBigInt<{ outer: { inner: { value: number } } }>(json);

      expect(result).toEqual({ outer: { inner: { value: 123 } } });
    });

    it('should parse empty objects and arrays', () => {
      expect(parseJsonWithBigInt('{}')).toEqual({});
      expect(parseJsonWithBigInt('[]')).toEqual([]);
    });

    it('should handle null and boolean values', () => {
      const json = '{"isActive": true, "data": null, "disabled": false}';
      const result = parseJsonWithBigInt<{ isActive: boolean; data: null; disabled: boolean }>(
        json
      );

      expect(result).toEqual({ isActive: true, data: null, disabled: false });
    });

    it('should preserve floating point numbers', () => {
      const json = '{"pi": 3.14159, "negative": -2.5, "scientific": 1.5e10}';
      const result = parseJsonWithBigInt<{ pi: number; negative: number; scientific: number }>(
        json
      );

      expect(result.pi).toBeCloseTo(3.14159);
      expect(result.negative).toBeCloseTo(-2.5);
      expect(result.scientific).toBeCloseTo(1.5e10);
    });

    it('should preserve string values that look like numbers', () => {
      const json = '{"id": "12345678901234567890"}';
      const result = parseJsonWithBigInt<{ id: string }>(json);

      expect(result.id).toBe('12345678901234567890');
      expect(typeof result.id).toBe('string');
    });
  });

  describe('BigInt conversion', () => {
    it('should convert integers larger than MAX_SAFE_INTEGER to BigInt', () => {
      const largeInt = '9007199254740993'; // MAX_SAFE_INTEGER + 2
      const json = `{"timestamp": ${largeInt}}`;
      const result = parseJsonWithBigInt<{ timestamp: bigint }>(json);

      expect(result.timestamp).toBe(BigInt(largeInt));
      expect(typeof result.timestamp).toBe('bigint');
    });

    it('should convert negative large integers to BigInt', () => {
      const largeNegative = '-9007199254740993';
      const json = `{"value": ${largeNegative}}`;
      const result = parseJsonWithBigInt<{ value: bigint }>(json);

      expect(result.value).toBe(BigInt(largeNegative));
    });

    it('should handle very large integers (nanosecond timestamps)', () => {
      // Typical nanosecond timestamp: 1704067200000000000 (Jan 1, 2024)
      const nanoTimestamp = '1704067200000000000';
      const json = `{"start": ${nanoTimestamp}, "end": ${nanoTimestamp}}`;
      const result = parseJsonWithBigInt<{ start: bigint; end: bigint }>(json);

      expect(result.start).toBe(BigInt(nanoTimestamp));
      expect(result.end).toBe(BigInt(nanoTimestamp));
    });

    it('should handle BigInt values in arrays', () => {
      const json = '[9007199254740993, 9007199254740994, 9007199254740995]';
      const result = parseJsonWithBigInt<bigint[]>(json);

      expect(result).toEqual([
        BigInt('9007199254740993'),
        BigInt('9007199254740994'),
        BigInt('9007199254740995'),
      ]);
    });

    it('should handle BigInt values in nested structures', () => {
      const json = `{
        "span": {
          "start": 1704067200000000000,
          "end": 1704153600000000000
        },
        "uses": [
          {"timestamp": 1704067200000000001}
        ]
      }`;
      const result = parseJsonWithBigInt<{
        span: { start: bigint; end: bigint };
        uses: { timestamp: bigint }[];
      }>(json);

      expect(result.span.start).toBe(BigInt('1704067200000000000'));
      expect(result.span.end).toBe(BigInt('1704153600000000000'));
      expect(result.uses[0].timestamp).toBe(BigInt('1704067200000000001'));
    });
  });

  describe('edge cases around MAX_SAFE_INTEGER', () => {
    it('should keep MAX_SAFE_INTEGER as Number', () => {
      const maxSafe = '9007199254740991'; // Number.MAX_SAFE_INTEGER
      const json = `{"value": ${maxSafe}}`;
      const result = parseJsonWithBigInt<{ value: number }>(json);

      expect(result.value).toBe(Number.MAX_SAFE_INTEGER);
      expect(typeof result.value).toBe('number');
    });

    it('should convert MAX_SAFE_INTEGER + 1 to BigInt', () => {
      const unsafe = '9007199254740992'; // MAX_SAFE_INTEGER + 1
      const json = `{"value": ${unsafe}}`;
      const result = parseJsonWithBigInt<{ value: bigint }>(json);

      expect(result.value).toBe(BigInt(unsafe));
      expect(typeof result.value).toBe('bigint');
    });

    it('should keep small 16-digit numbers as Number if safe', () => {
      // 1000000000000000 is 16 digits but safe
      const safeValue = '1000000000000000';
      const json = `{"value": ${safeValue}}`;
      const result = parseJsonWithBigInt<{ value: number }>(json);

      expect(result.value).toBe(1000000000000000);
      expect(typeof result.value).toBe('number');
    });
  });

  describe('mixed content', () => {
    it('should handle objects with both safe and unsafe integers', () => {
      const json = `{
        "id": 12345,
        "timestamp": 1704067200000000000,
        "count": 999,
        "nanos": 9007199254740993
      }`;
      const result = parseJsonWithBigInt<{
        id: number;
        timestamp: bigint;
        count: number;
        nanos: bigint;
      }>(json);

      expect(result.id).toBe(12345);
      expect(typeof result.id).toBe('number');
      expect(result.timestamp).toBe(BigInt('1704067200000000000'));
      expect(typeof result.timestamp).toBe('bigint');
      expect(result.count).toBe(999);
      expect(result.nanos).toBe(BigInt('9007199254740993'));
    });

    it('should handle arrays with mixed safe and unsafe integers', () => {
      const json = '[1, 9007199254740993, 2, 9007199254740994, 3]';
      const result = parseJsonWithBigInt<(number | bigint)[]>(json);

      expect(result[0]).toBe(1);
      expect(result[1]).toBe(BigInt('9007199254740993'));
      expect(result[2]).toBe(2);
      expect(result[3]).toBe(BigInt('9007199254740994'));
      expect(result[4]).toBe(3);
    });
  });

  describe('real-world scenarios', () => {
    it('should parse a ResourceTimeline-like response', () => {
      const json = `{
        "span": {
          "start": 1704067200000000000,
          "end": 1704153600000000000
        },
        "uses": [
          {
            "span": {"start": 1704067200000000000, "end": 1704070800000000000},
            "amounts": [{"key": "bytes", "value": {"U64": 1024}}],
            "entity": {"Worker": "worker-1"}
          }
        ]
      }`;
      const result = parseJsonWithBigInt<{
        span: { start: bigint; end: bigint };
        uses: {
          span: { start: bigint; end: bigint };
          amounts: { key: string; value: { U64: number } }[];
          entity: { Worker: string };
        }[];
      }>(json);

      expect(result.span.start).toBe(BigInt('1704067200000000000'));
      expect(result.span.end).toBe(BigInt('1704153600000000000'));
      expect(result.uses[0].span.start).toBe(BigInt('1704067200000000000'));
      expect(result.uses[0].amounts[0].key).toBe('bytes');
      expect(result.uses[0].entity.Worker).toBe('worker-1');
    });

    it('should handle consecutive BigInt values', () => {
      const json = '[9007199254740993,9007199254740994,9007199254740995]';
      const result = parseJsonWithBigInt<bigint[]>(json);

      expect(result).toEqual([
        BigInt('9007199254740993'),
        BigInt('9007199254740994'),
        BigInt('9007199254740995'),
      ]);
    });
  });
});
