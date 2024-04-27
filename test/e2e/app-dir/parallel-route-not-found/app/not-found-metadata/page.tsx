import { notFound } from 'next/navigation'

export function generateMetadata() {
  notFound()
  return {
    title: 'Create Next App',
  }
}

export default function Page() {
  return <h1>Hello from Page</h1>
}
