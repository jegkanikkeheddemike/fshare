import type { Key, ReactNode } from "react";

export const Disabled = (props: { disabled: boolean, children: ReactNode, key?: Key }) => {
    if (props.disabled) {
        return <div className="opacity-50 pointer-events-none" key={props.key}>
            {props.children}
        </div>
    }
    return <div key={props.key}>
        {props.children}
    </div>
}