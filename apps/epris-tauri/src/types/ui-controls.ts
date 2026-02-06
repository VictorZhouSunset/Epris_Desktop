import { z } from 'zod';

export const EPRIS_ID_RE = /^[A-Za-z_$][A-Za-z0-9_$]*$/;

export type EprisControlType = 'number' | 'color' | 'select' | 'boolean' | 'text';

export type EprisControlSpec =
  | {
      id: string;
      label: string;
      objectId?: string;
      type: 'number';
      ui?: 'slider' | 'input';
      group?: string;
      min?: number;
      max?: number;
      step?: number;
    }
  | {
      id: string;
      label: string;
      objectId?: string;
      type: 'color';
      ui?: 'color';
      group?: string;
    }
  | {
      id: string;
      label: string;
      objectId?: string;
      type: 'select';
      ui?: 'select';
      group?: string;
      options: Array<string | { value: string; label?: string }>;
    }
  | {
      id: string;
      label: string;
      objectId?: string;
      type: 'boolean';
      ui?: 'toggle';
      group?: string;
    }
  | {
      id: string;
      label: string;
      objectId?: string;
      type: 'text';
      ui?: 'text';
      group?: string;
      placeholder?: string;
    };

export type EprisControlsSpec = {
  schemaVersion: 1;
  target?: { kind: 'composition'; id: string } | { kind: 'global' };
  controls: EprisControlSpec[];
};

export type EprisPropsFile = {
  schemaVersion: 1;
  values: Record<string, unknown>;
};

const ControlBase = z.object({
  id: z.string().regex(EPRIS_ID_RE, 'id must be a valid JS identifier'),
  label: z.string().min(1),
  objectId: z.string().regex(EPRIS_ID_RE, 'objectId must be a valid JS identifier').optional(),
  group: z.string().min(1).optional(),
});

const NumberControl = ControlBase.extend({
  type: z.literal('number'),
  ui: z.enum(['slider', 'input']).optional(),
  min: z.number().optional(),
  max: z.number().optional(),
  step: z.number().optional(),
});

const ColorControl = ControlBase.extend({
  type: z.literal('color'),
  ui: z.literal('color').optional(),
});

const SelectControl = ControlBase.extend({
  type: z.literal('select'),
  ui: z.literal('select').optional(),
  options: z.array(z.union([z.string(), z.object({ value: z.string(), label: z.string().optional() })])).min(1),
});

const BooleanControl = ControlBase.extend({
  type: z.literal('boolean'),
  ui: z.literal('toggle').optional(),
});

const TextControl = ControlBase.extend({
  type: z.literal('text'),
  ui: z.literal('text').optional(),
  placeholder: z.string().optional(),
});

export const EprisControlSpecSchema = z.discriminatedUnion('type', [
  NumberControl,
  ColorControl,
  SelectControl,
  BooleanControl,
  TextControl,
]);

export const EprisControlsSpecSchema: z.ZodType<EprisControlsSpec> = z.object({
  schemaVersion: z.literal(1),
  target: z
    .union([
      z.object({ kind: z.literal('composition'), id: z.string().min(1) }),
      z.object({ kind: z.literal('global') }),
    ])
    .optional(),
  controls: z.array(EprisControlSpecSchema),
});

export const EprisPropsFileSchema: z.ZodType<EprisPropsFile> = z
  .object({
    schemaVersion: z.literal(1),
    values: z.record(z.string(), z.unknown()),
  })
  .superRefine((val, ctx) => {
    for (const key of Object.keys(val.values)) {
      if (!EPRIS_ID_RE.test(key)) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: `values key '${key}' must be a valid JS identifier`,
          path: ['values', key],
        });
      }
    }
  });

export function formatZodError(err: z.ZodError): string {
  const lines: string[] = [];
  for (const issue of err.issues) {
    const p = issue.path.length ? issue.path.join('.') : '(root)';
    lines.push(`${p}: ${issue.message}`);
  }
  return lines.join('\n');
}
