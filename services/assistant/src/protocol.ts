import {Type, type Static} from '@sinclair/typebox';
import {Value} from '@sinclair/typebox/value';
const id=Type.String({pattern:'^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'});
const object=<T extends Record<string, import('@sinclair/typebox').TSchema>>(properties:T)=>Type.Object(properties,{additionalProperties:false});
export const RequestSchema=Type.Union([
  object({action:Type.Literal('status')}), object({action:Type.Literal('list')}),
  object({action:Type.Literal('create'),commandId:id}),
  object({action:Type.Literal('snapshot'),conversationId:id}),
  object({action:Type.Literal('send'),conversationId:id,commandId:id,text:Type.String({minLength:1,maxLength:4096})}),
  object({action:Type.Literal('stop'),conversationId:id,turnId:id}),
  object({action:Type.Literal('recover'),conversationId:id,turnId:id}),
],{$id:'shadows-local-assistant-request-v1'});
export type Request=Static<typeof RequestSchema>;
export function parseRequest(value:unknown):Request {
  if(!Value.Check(RequestSchema,value)||value.action==='send'&&(!value.text.trim()||Buffer.byteLength(value.text)>4096))throw new Error('request_invalid');
  return value;
}
export const safeErrors=new Set(['request_invalid','command_conflict','generation_busy','not_found','unavailable','outcome_unknown']);
